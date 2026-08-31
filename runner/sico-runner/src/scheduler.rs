//! Bounded single-Store cooperative scheduler core (M11 STEP-0105).
//!
//! One `SchedulerCore` lives inside one fresh Store per top-level run
//! (ADR-0010). It owns the run's task table, scope tree, FIFO ready queue,
//! Host completion ingress and cumulative run budget. Guest code keeps the
//! sequential-v1 profile (RFC-0036 §5.4): source-level tasks complete
//! eagerly inside the guest, so host-side the root task plus registered
//! Host operations are the managed entities in v1. The identity, bounds
//! and ingress discipline frozen here carry over to any future suspension
//! profile unchanged.
//!
//! Canonical same-turn ordering (ADR-0010 §6.3): readiness class, then
//! registration order, then task id. Ids increase monotonically and never
//! repeat within a run; nothing from one generation resolves in another.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;

/// Live tasks per run (ADR-0010 hard limits).
pub const MAX_LIVE_TASKS: usize = 1_024;
/// Task-scope nesting depth.
pub const MAX_SCOPE_DEPTH: u32 = 64;
/// Runnable queue entries.
pub const MAX_READY_QUEUE: usize = 1_024;
/// Host completion queue: records and payload bytes.
pub const MAX_COMPLETION_RECORDS: usize = 1_024;
pub const MAX_COMPLETION_BYTES: usize = 16 << 20;
/// Scheduler and task metadata budget per run.
pub const MAX_METADATA_BYTES: usize = 16 << 20;
/// Children of one task group.
pub const MAX_SCOPE_CHILDREN: usize = 1_024;

/// Accounted sizes for the metadata budget. With every count capped at
/// 1,024 the budget is structurally unreachable (hundreds of KiB at most);
/// it is enforced anyway so a future larger record cannot grow silently.
const TASK_RECORD_BYTES: usize = 128;
const SCOPE_RECORD_BYTES: usize = 96;
const READY_ENTRY_BYTES: usize = 16;
const OPERATION_RECORD_BYTES: usize = 96;

/// Run-local task identity; monotonically increasing within one run.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TaskId(pub u32);

/// Run-local task-scope identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScopeId(pub u32);

/// Run-local Host-operation registration identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationId(pub u64);

/// Task lifecycle (ADR-0010 §6.1):
/// `created → runnable ↔ suspended → completing → succeeded | failed | cancelled`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskState {
    Created,
    Runnable,
    Suspended,
    Completing,
    Succeeded,
    Failed,
    Cancelled,
}

impl TaskState {
    /// Terminal states commit exactly once and never leave.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

/// How a task's terminal state was reached.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalKind {
    Succeeded,
    Failed,
    Cancelled,
}

/// Readiness classes in canonical turn order (lowest first).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReadinessClass {
    /// Cancellation is always observed before ordinary readiness.
    Cancellation,
    /// A Host-operation completion record.
    HostCompletion,
    /// A timer/deadline wait becoming due.
    Timer,
}

/// One bounded Host completion record (ADR-0010 §7.2): run, generation,
/// task and operation identity plus bounded payload bytes. The readiness
/// class is the one fixed at operation registration, not caller-supplied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionRecord {
    pub run_id: String,
    pub generation_id: u64,
    pub task: TaskId,
    pub operation: OperationId,
    pub payload_bytes: usize,
}

/// Stable typed scheduler outcomes; no limit is ever silently clamped and
/// no path panics on adversarial input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchedulerFault {
    TaskTableFull,
    ScopeChildrenExceeded,
    ScopeDepthExceeded,
    ReadyQueueFull,
    CompletionQueueFull,
    CompletionBytesExceeded,
    MetadataBudgetExceeded,
    UnknownTask(TaskId),
    UnknownScope(ScopeId),
    UnknownOperation(OperationId),
    ScopeNotOpen(ScopeId),
    IllegalTransition {
        task: TaskId,
        from: TaskState,
        to: TaskState,
    },
    TaskNotTerminal(TaskId),
    CrossRunCompletion,
    StaleCompletion(OperationId),
    DuplicateCompletion(OperationId),
    HostOperationNotTerminal(OperationId),
    PayloadOutsideBudget,
}

impl fmt::Display for SchedulerFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskTableFull => formatter.write_str("task table full (1024 live tasks)"),
            Self::ScopeChildrenExceeded => {
                formatter.write_str("task-group children exceeded (1024)")
            }
            Self::ScopeDepthExceeded => formatter.write_str("scope nesting exceeded (64)"),
            Self::ReadyQueueFull => formatter.write_str("ready queue full (1024)"),
            Self::CompletionQueueFull => {
                formatter.write_str("host completion queue full (1024 records)")
            }
            Self::CompletionBytesExceeded => {
                formatter.write_str("host completion bytes exceeded (16 MiB)")
            }
            Self::MetadataBudgetExceeded => {
                formatter.write_str("scheduler metadata budget exceeded (16 MiB)")
            }
            Self::UnknownTask(task) => write!(formatter, "unknown task {}", task.0),
            Self::UnknownScope(scope) => write!(formatter, "unknown scope {}", scope.0),
            Self::UnknownOperation(operation) => {
                write!(formatter, "unknown operation {}", operation.0)
            }
            Self::ScopeNotOpen(scope) => write!(formatter, "scope {} is not open", scope.0),
            Self::IllegalTransition { task, from, to } => write!(
                formatter,
                "illegal task {} transition {from:?} -> {to:?}",
                task.0
            ),
            Self::TaskNotTerminal(task) => {
                write!(formatter, "task {} is not terminal at teardown", task.0)
            }
            Self::CrossRunCompletion => {
                formatter.write_str("completion record belongs to another run or generation")
            }
            Self::StaleCompletion(operation) => {
                write!(
                    formatter,
                    "completion for terminal/unknown operation {}",
                    operation.0
                )
            }
            Self::DuplicateCompletion(operation) => {
                write!(
                    formatter,
                    "duplicate completion for operation {}",
                    operation.0
                )
            }
            Self::HostOperationNotTerminal(operation) => write!(
                formatter,
                "host operation {} is not terminal at teardown",
                operation.0
            ),
            Self::PayloadOutsideBudget => {
                formatter.write_str("completion payload outside the per-record budget")
            }
        }
    }
}

impl Error for SchedulerFault {}

#[derive(Clone, Copy, Debug)]
struct TaskRecord {
    scope: ScopeId,
    parent: Option<TaskId>,
    state: TaskState,
}

#[derive(Clone, Debug)]
struct ScopeRecord {
    parent: Option<ScopeId>,
    depth: u32,
    children: BTreeSet<TaskId>,
    open: bool,
}

#[derive(Clone, Copy, Debug)]
struct OperationRecord {
    task: TaskId,
    class: ReadinessClass,
    terminal: bool,
}

/// One run's deterministic identity scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunIdentity {
    pub run_id: String,
    pub generation_id: u64,
}

/// The bounded scheduler core owned by one fresh Store.
#[derive(Debug)]
pub struct SchedulerCore {
    identity: RunIdentity,
    tasks: BTreeMap<TaskId, TaskRecord>,
    scopes: BTreeMap<ScopeId, ScopeRecord>,
    ready: VecDeque<TaskId>,
    queued: BTreeSet<TaskId>,
    operations: BTreeMap<OperationId, OperationRecord>,
    completions: Vec<CompletionRecord>,
    delivered: BTreeSet<OperationId>,
    completion_bytes: usize,
    registration_order: u64,
    operation_order: BTreeMap<OperationId, u64>,
    next_task: u32,
    next_scope: u32,
    next_operation: u64,
    metadata_bytes: usize,
    /// Cumulative run-level accounting: spawning never multiplies a budget.
    pub host_operations_total: u64,
    torn_down: bool,
}

impl SchedulerCore {
    /// Creates the scheduler for one run: root scope 0 plus root task 1 in
    /// `created` state, ready to be made runnable at guest launch.
    pub fn new(identity: RunIdentity) -> Self {
        let mut scheduler = Self {
            identity,
            tasks: BTreeMap::new(),
            scopes: BTreeMap::new(),
            ready: VecDeque::new(),
            queued: BTreeSet::new(),
            operations: BTreeMap::new(),
            completions: Vec::new(),
            delivered: BTreeSet::new(),
            completion_bytes: 0,
            registration_order: 0,
            operation_order: BTreeMap::new(),
            next_task: 1,
            next_scope: 1,
            next_operation: 1,
            metadata_bytes: 0,
            host_operations_total: 0,
            torn_down: false,
        };
        scheduler.metadata_bytes += SCOPE_RECORD_BYTES + TASK_RECORD_BYTES;
        scheduler.scopes.insert(
            ScopeId(0),
            ScopeRecord {
                parent: None,
                depth: 1,
                children: BTreeSet::new(),
                open: true,
            },
        );
        scheduler.tasks.insert(
            TaskId(0),
            TaskRecord {
                scope: ScopeId(0),
                parent: None,
                state: TaskState::Created,
            },
        );
        scheduler
            .scopes
            .get_mut(&ScopeId(0))
            .expect("root scope")
            .children
            .insert(TaskId(0));
        scheduler
    }

    /// The run identity every completion record must match.
    pub fn identity(&self) -> &RunIdentity {
        &self.identity
    }

    /// The root task of the run (id 0 in the root scope).
    pub fn root_task(&self) -> TaskId {
        TaskId(0)
    }

    pub fn task_state(&self, task: TaskId) -> Option<TaskState> {
        self.tasks.get(&task).map(|record| record.state)
    }

    pub fn live_tasks(&self) -> usize {
        self.tasks
            .values()
            .filter(|record| !record.state.is_terminal())
            .count()
    }

    /// The task's parent in the ownership graph (ADR-0010: at most one
    /// parent; the downward cancellation tree consumes this in STEP-0106).
    pub fn parent_of(&self, task: TaskId) -> Option<TaskId> {
        self.tasks.get(&task).and_then(|record| record.parent)
    }

    /// Opens a child scope; depth is proven from the parent chain.
    pub fn open_scope(&mut self, parent: ScopeId) -> Result<ScopeId, SchedulerFault> {
        let parent_record = self
            .scopes
            .get(&parent)
            .ok_or(SchedulerFault::UnknownScope(parent))?;
        if !parent_record.open {
            return Err(SchedulerFault::ScopeNotOpen(parent));
        }
        let depth = parent_record.depth + 1;
        if depth > MAX_SCOPE_DEPTH {
            return Err(SchedulerFault::ScopeDepthExceeded);
        }
        self.charge_metadata(SCOPE_RECORD_BYTES)?;
        let scope = ScopeId(self.next_scope);
        self.next_scope += 1;
        self.scopes.insert(
            scope,
            ScopeRecord {
                parent: Some(parent),
                depth,
                children: BTreeSet::new(),
                open: true,
            },
        );
        Ok(scope)
    }

    /// Closes a scope once every child task is terminal (ADR-0010: scope
    /// exit collects or cancels every child).
    pub fn close_scope(&mut self, scope: ScopeId) -> Result<(), SchedulerFault> {
        let record = self
            .scopes
            .get(&scope)
            .ok_or(SchedulerFault::UnknownScope(scope))?;
        if !record.open {
            return Err(SchedulerFault::ScopeNotOpen(scope));
        }
        if let Some(task) = record
            .children
            .iter()
            .find(|task| {
                self.tasks
                    .get(task)
                    .is_some_and(|child| !child.state.is_terminal())
            })
            .copied()
        {
            return Err(SchedulerFault::TaskNotTerminal(task));
        }
        self.scopes.get_mut(&scope).expect("scope checked").open = false;
        Ok(())
    }

    /// Spawns a task into an open scope; the parent task must live in the
    /// same scope and be non-terminal.
    pub fn spawn_task(&mut self, scope: ScopeId, parent: TaskId) -> Result<TaskId, SchedulerFault> {
        let scope_record = self
            .scopes
            .get(&scope)
            .ok_or(SchedulerFault::UnknownScope(scope))?;
        if !scope_record.open {
            return Err(SchedulerFault::ScopeNotOpen(scope));
        }
        if scope_record.children.len() >= MAX_SCOPE_CHILDREN {
            return Err(SchedulerFault::ScopeChildrenExceeded);
        }
        let parent_record = self
            .tasks
            .get(&parent)
            .ok_or(SchedulerFault::UnknownTask(parent))?;
        // The spawning task lives in an ancestor-or-self scope of the new
        // task's scope (a `task group` nests inside its parent's extent).
        if parent_record.state.is_terminal() || !self.scope_contains(parent_record.scope, scope) {
            return Err(SchedulerFault::IllegalTransition {
                task: parent,
                from: parent_record.state,
                to: TaskState::Created,
            });
        }
        if self.live_tasks() >= MAX_LIVE_TASKS {
            return Err(SchedulerFault::TaskTableFull);
        }
        self.charge_metadata(TASK_RECORD_BYTES)?;
        let task = TaskId(self.next_task);
        self.next_task += 1;
        self.tasks.insert(
            task,
            TaskRecord {
                scope,
                parent: Some(parent),
                state: TaskState::Created,
            },
        );
        self.scopes
            .get_mut(&scope)
            .expect("scope checked")
            .children
            .insert(task);
        Ok(task)
    }

    /// `created | suspended → runnable`: enqueue at the FIFO tail.
    pub fn make_runnable(&mut self, task: TaskId) -> Result<(), SchedulerFault> {
        let record = self
            .tasks
            .get(&task)
            .ok_or(SchedulerFault::UnknownTask(task))?;
        if !matches!(record.state, TaskState::Created | TaskState::Suspended) {
            return Err(SchedulerFault::IllegalTransition {
                task,
                from: record.state,
                to: TaskState::Runnable,
            });
        }
        if self.ready.len() >= MAX_READY_QUEUE && !self.queued.contains(&task) {
            return Err(SchedulerFault::ReadyQueueFull);
        }
        self.tasks.get_mut(&task).expect("task checked").state = TaskState::Runnable;
        // A suspended task keeps its (now stale) queue slot; requeueing it
        // must not create a duplicate entry.
        if self.queued.insert(task) {
            self.charge_metadata(READY_ENTRY_BYTES)?;
            self.ready.push_back(task);
        }
        Ok(())
    }

    /// Pops the FIFO head that is still runnable (stale entries from
    /// cancelled/suspended tasks are skipped exactly once).
    pub fn next_ready(&mut self) -> Option<TaskId> {
        while let Some(task) = self.ready.pop_front() {
            self.queued.remove(&task);
            if self.tasks.get(&task).map(|record| record.state) == Some(TaskState::Runnable) {
                return Some(task);
            }
        }
        None
    }

    /// `runnable → suspended` at a cooperative yield point.
    pub fn suspend(&mut self, task: TaskId) -> Result<(), SchedulerFault> {
        self.transition(task, TaskState::Runnable, TaskState::Suspended)
    }

    /// `runnable → completing`: the task is producing its terminal value.
    pub fn begin_completion(&mut self, task: TaskId) -> Result<(), SchedulerFault> {
        self.transition(task, TaskState::Runnable, TaskState::Completing)
    }

    /// Commits the task's single terminal state.
    pub fn commit_terminal(
        &mut self,
        task: TaskId,
        kind: TerminalKind,
    ) -> Result<(), SchedulerFault> {
        let record = self
            .tasks
            .get(&task)
            .ok_or(SchedulerFault::UnknownTask(task))?;
        if record.state.is_terminal() {
            return Err(SchedulerFault::IllegalTransition {
                task,
                from: record.state,
                to: match kind {
                    TerminalKind::Succeeded => TaskState::Succeeded,
                    TerminalKind::Failed => TaskState::Failed,
                    TerminalKind::Cancelled => TaskState::Cancelled,
                },
            });
        }
        self.tasks.get_mut(&task).expect("task checked").state = match kind {
            TerminalKind::Succeeded => TaskState::Succeeded,
            TerminalKind::Failed => TaskState::Failed,
            TerminalKind::Cancelled => TaskState::Cancelled,
        };
        Ok(())
    }

    /// Registers one Host operation against a live task and returns its
    /// monotonic operation identity (charged to the run, not the task).
    pub fn register_operation(
        &mut self,
        task: TaskId,
        class: ReadinessClass,
    ) -> Result<OperationId, SchedulerFault> {
        let record = self
            .tasks
            .get(&task)
            .ok_or(SchedulerFault::UnknownTask(task))?;
        if record.state.is_terminal() {
            return Err(SchedulerFault::IllegalTransition {
                task,
                from: record.state,
                to: TaskState::Runnable,
            });
        }
        self.charge_metadata(OPERATION_RECORD_BYTES)?;
        let operation = OperationId(self.next_operation);
        self.next_operation += 1;
        self.registration_order += 1;
        self.operation_order
            .insert(operation, self.registration_order);
        self.operations.insert(
            operation,
            OperationRecord {
                task,
                class,
                terminal: false,
            },
        );
        self.host_operations_total += 1;
        Ok(operation)
    }

    /// Publishes one bounded Host completion record through the identity
    /// ingress. Stale, duplicate, late and cross-run/generation records are
    /// rejected closed and never wake a task.
    pub fn publish_completion(&mut self, record: CompletionRecord) -> Result<(), SchedulerFault> {
        if self.torn_down
            || record.run_id != self.identity.run_id
            || record.generation_id != self.identity.generation_id
        {
            return Err(SchedulerFault::CrossRunCompletion);
        }
        let operation = self
            .operations
            .get(&record.operation)
            .copied()
            .ok_or(SchedulerFault::UnknownOperation(record.operation))?;
        if operation.task != record.task {
            return Err(SchedulerFault::StaleCompletion(record.operation));
        }
        if self.delivered.contains(&record.operation) {
            return Err(SchedulerFault::DuplicateCompletion(record.operation));
        }
        if operation.terminal {
            return Err(SchedulerFault::StaleCompletion(record.operation));
        }
        if self.completions.len() >= MAX_COMPLETION_RECORDS {
            return Err(SchedulerFault::CompletionQueueFull);
        }
        if self.completion_bytes + record.payload_bytes > MAX_COMPLETION_BYTES {
            return Err(SchedulerFault::CompletionBytesExceeded);
        }
        self.completion_bytes += record.payload_bytes;
        self.operations
            .get_mut(&record.operation)
            .expect("operation checked")
            .terminal = true;
        self.delivered.insert(record.operation);
        self.completions.push(record);
        Ok(())
    }

    /// Drains the completions observed in one turn in canonical order:
    /// readiness class (fixed at registration), registration order, then
    /// task id (ADR-0010 §6.3).
    pub fn drain_turn(&mut self) -> Vec<CompletionRecord> {
        let mut turn = std::mem::take(&mut self.completions);
        turn.sort_by_key(|record| {
            (
                self.operations
                    .get(&record.operation)
                    .map(|operation| operation.class),
                self.operation_order
                    .get(&record.operation)
                    .copied()
                    .unwrap_or(u64::MAX),
                record.task,
            )
        });
        turn
    }

    /// Registers a Host operation on the root task, the only host-visible
    /// task in the sequential-v1 profile (RFC-0036 §5.4).
    pub fn register_root_operation(
        &mut self,
        class: ReadinessClass,
    ) -> Result<OperationId, SchedulerFault> {
        self.register_operation(self.root_task(), class)
    }

    /// Publishes the completion of a root-task operation with the exact run
    /// identity filled in by the scheduler itself.
    pub fn complete_root_operation(
        &mut self,
        operation: OperationId,
        payload_bytes: usize,
    ) -> Result<(), SchedulerFault> {
        self.publish_completion(CompletionRecord {
            run_id: self.identity.run_id.clone(),
            generation_id: self.identity.generation_id,
            task: self.root_task(),
            operation,
            payload_bytes,
        })
    }

    /// Classifies an abandoned Host operation (cancel/timeout with an
    /// unjoinable worker) as terminal without delivering a completion; a
    /// later record for it is stale (ADR-0010 adversarial traces 3–4).
    pub fn abandon_operation(&mut self, operation: OperationId) -> Result<(), SchedulerFault> {
        self.operations
            .get_mut(&operation)
            .ok_or(SchedulerFault::UnknownOperation(operation))?
            .terminal = true;
        Ok(())
    }

    /// Commits the root task's single terminal state from the run outcome.
    pub fn commit_root(&mut self, kind: TerminalKind) -> Result<(), SchedulerFault> {
        let root = self.root_task();
        if self.task_state(root) == Some(TaskState::Runnable) {
            self.begin_completion(root)?;
        }
        self.commit_terminal(root, kind)
    }

    /// Tears the scheduler down before the Store drops: every task must be
    /// terminal; scopes close in deterministic reverse order (deepest and
    /// highest-id first), and the ingress stops accepting records.
    pub fn teardown(&mut self) -> Result<Vec<ScopeId>, SchedulerFault> {
        if let Some((task, _)) = self
            .tasks
            .iter()
            .find(|(_, record)| !record.state.is_terminal())
        {
            return Err(SchedulerFault::TaskNotTerminal(*task));
        }
        if let Some((operation, _)) = self.operations.iter().find(|(_, record)| !record.terminal) {
            return Err(SchedulerFault::HostOperationNotTerminal(*operation));
        }
        self.completions.clear();
        let mut order: Vec<_> = self
            .scopes
            .iter()
            .filter(|(_, record)| record.open)
            .map(|(scope, record)| (*scope, record.depth))
            .collect();
        order.sort_by_key(|(scope, depth)| (Reverse(*depth), Reverse(scope.0)));
        let mut closed = Vec::with_capacity(order.len());
        for (scope, _) in order {
            self.close_scope(scope)?;
            closed.push(scope);
        }
        self.torn_down = true;
        Ok(closed)
    }

    /// Exact accounted scheduler metadata bytes (table, queues, operations).
    pub fn metadata_bytes(&self) -> usize {
        self.metadata_bytes
    }

    /// True when `ancestor` is `scope` itself or one of its parents.
    fn scope_contains(&self, ancestor: ScopeId, scope: ScopeId) -> bool {
        let mut current = Some(scope);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.scopes.get(&id).and_then(|record| record.parent);
        }
        false
    }

    fn transition(
        &mut self,
        task: TaskId,
        from: TaskState,
        to: TaskState,
    ) -> Result<(), SchedulerFault> {
        let record = self
            .tasks
            .get(&task)
            .ok_or(SchedulerFault::UnknownTask(task))?;
        if record.state != from {
            return Err(SchedulerFault::IllegalTransition {
                task,
                from: record.state,
                to,
            });
        }
        self.tasks.get_mut(&task).expect("task checked").state = to;
        Ok(())
    }

    fn charge_metadata(&mut self, bytes: usize) -> Result<(), SchedulerFault> {
        if self.metadata_bytes + bytes > MAX_METADATA_BYTES {
            return Err(SchedulerFault::MetadataBudgetExceeded);
        }
        self.metadata_bytes += bytes;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheduler() -> SchedulerCore {
        SchedulerCore::new(RunIdentity {
            run_id: "run-test".to_owned(),
            generation_id: 1,
        })
    }

    fn record(scheduler: &SchedulerCore, task: TaskId, operation: OperationId) -> CompletionRecord {
        CompletionRecord {
            run_id: scheduler.identity.run_id.clone(),
            generation_id: scheduler.identity.generation_id,
            task,
            operation,
            payload_bytes: 0,
        }
    }

    #[test]
    fn lifecycle_accepts_the_documented_paths() {
        let mut scheduler = scheduler();
        let root = scheduler.root_task();
        assert_eq!(scheduler.task_state(root), Some(TaskState::Created));
        // A created task cannot skip straight to completing.
        scheduler.begin_completion(root).unwrap_err();
        scheduler.make_runnable(root).unwrap();
        assert_eq!(scheduler.next_ready(), Some(root));
        // Popping leaves the task runnable; the cooperative cycle works.
        scheduler.suspend(root).unwrap();
        scheduler.make_runnable(root).unwrap();
        assert_eq!(scheduler.next_ready(), Some(root));
        scheduler.begin_completion(root).unwrap();
        scheduler
            .commit_terminal(root, TerminalKind::Succeeded)
            .unwrap();
        // Terminal commits exactly once.
        scheduler
            .commit_terminal(root, TerminalKind::Failed)
            .unwrap_err();
        assert_eq!(scheduler.live_tasks(), 0);
    }

    #[test]
    fn spawn_fill_and_limit_plus_one_are_typed() {
        let mut scheduler = scheduler();
        let root = scheduler.root_task();
        scheduler.make_runnable(root).unwrap();
        let _ = scheduler.next_ready();
        // Two scopes keep each scope under the children cap so the task
        // table bound is the one that trips: root + 511 + 512 = 1,024.
        let child_scope = scheduler.open_scope(ScopeId(0)).unwrap();
        for _ in 0..511 {
            scheduler.spawn_task(ScopeId(0), root).unwrap();
        }
        for _ in 0..512 {
            scheduler.spawn_task(child_scope, root).unwrap();
        }
        assert_eq!(scheduler.live_tasks(), MAX_LIVE_TASKS);
        assert_eq!(
            scheduler.spawn_task(ScopeId(0), root),
            Err(SchedulerFault::TaskTableFull)
        );
    }

    #[test]
    fn scope_children_limit_is_typed() {
        let mut scheduler = scheduler();
        let root = scheduler.root_task();
        // Scope 0 already holds the root; 1,023 more children fill it.
        for _ in 1..MAX_SCOPE_CHILDREN {
            scheduler.spawn_task(ScopeId(0), root).unwrap();
        }
        assert_eq!(
            scheduler.spawn_task(ScopeId(0), root),
            Err(SchedulerFault::ScopeChildrenExceeded)
        );
    }

    #[test]
    fn scope_depth_and_children_limits_are_typed() {
        let mut scheduler = scheduler();
        let mut scope = ScopeId(0);
        for _ in 1..MAX_SCOPE_DEPTH {
            scope = scheduler.open_scope(scope).unwrap();
        }
        assert_eq!(
            scheduler.open_scope(scope),
            Err(SchedulerFault::ScopeDepthExceeded)
        );
    }

    #[test]
    fn completion_ingress_rejects_stale_duplicate_and_cross_run_records() {
        let mut scheduler = scheduler();
        let root = scheduler.root_task();
        let operation = scheduler
            .register_operation(root, ReadinessClass::HostCompletion)
            .unwrap();
        scheduler
            .publish_completion(record(&scheduler, root, operation))
            .unwrap();
        assert_eq!(
            scheduler.publish_completion(record(&scheduler, root, operation)),
            Err(SchedulerFault::DuplicateCompletion(operation))
        );
        assert_eq!(
            scheduler.publish_completion(CompletionRecord {
                run_id: "run-other".to_owned(),
                ..record(&scheduler, root, OperationId(999))
            }),
            Err(SchedulerFault::CrossRunCompletion)
        );
        assert_eq!(
            scheduler.publish_completion(record(&scheduler, root, OperationId(999))),
            Err(SchedulerFault::UnknownOperation(OperationId(999)))
        );
        let late = scheduler
            .register_operation(root, ReadinessClass::Timer)
            .unwrap();
        scheduler
            .publish_completion(record(&scheduler, root, late))
            .unwrap();
        assert_eq!(
            scheduler.publish_completion(record(&scheduler, root, late)),
            Err(SchedulerFault::DuplicateCompletion(late))
        );
    }

    #[test]
    fn same_turn_readiness_follows_the_canonical_order() {
        let mut scheduler = scheduler();
        let root = scheduler.root_task();
        scheduler.make_runnable(root).unwrap();
        let _ = scheduler.next_ready();
        let mut tasks = Vec::new();
        for _ in 0..4 {
            let task = scheduler.spawn_task(ScopeId(0), root).unwrap();
            scheduler.make_runnable(task).unwrap();
            let _ = scheduler.next_ready();
            tasks.push(task);
        }
        // Register out of order across classes: host completions on tasks
        // 4/2, a timer on task 3, one cancellation on task 1.
        let host_late = scheduler
            .register_operation(tasks[3], ReadinessClass::HostCompletion)
            .unwrap();
        let timer = scheduler
            .register_operation(tasks[2], ReadinessClass::Timer)
            .unwrap();
        let cancel = scheduler
            .register_operation(tasks[0], ReadinessClass::Cancellation)
            .unwrap();
        let host_early = scheduler
            .register_operation(tasks[1], ReadinessClass::HostCompletion)
            .unwrap();
        for (task, operation) in [
            (tasks[3], host_late),
            (tasks[2], timer),
            (tasks[0], cancel),
            (tasks[1], host_early),
        ] {
            scheduler
                .publish_completion(record(&scheduler, task, operation))
                .unwrap();
        }
        let turn: Vec<_> = scheduler
            .drain_turn()
            .iter()
            .map(|record| record.operation)
            .collect();
        // Cancellation first; then host completions in registration order
        // (host_late registered before host_early); timer last.
        assert_eq!(turn, vec![cancel, host_late, host_early, timer]);
    }

    #[test]
    fn teardown_requires_terminal_tasks_and_closes_scopes_in_reverse() {
        let mut scheduler = scheduler();
        let root = scheduler.root_task();
        let child_scope = scheduler.open_scope(ScopeId(0)).unwrap();
        scheduler.make_runnable(root).unwrap();
        let _ = scheduler.next_ready();
        let child = scheduler.spawn_task(child_scope, root).unwrap();
        assert!(matches!(
            scheduler.teardown(),
            Err(SchedulerFault::TaskNotTerminal(_))
        ));
        scheduler.make_runnable(child).unwrap();
        let _ = scheduler.next_ready();
        scheduler.begin_completion(child).unwrap();
        scheduler
            .commit_terminal(child, TerminalKind::Cancelled)
            .unwrap();
        // A runnable task cannot be re-queued; after `begin_completion`,
        // suspension is illegal.
        scheduler.make_runnable(root).unwrap_err();
        scheduler.begin_completion(root).unwrap();
        scheduler.suspend(root).unwrap_err();
        scheduler
            .commit_terminal(root, TerminalKind::Succeeded)
            .unwrap();
        let closed = scheduler.teardown().unwrap();
        assert_eq!(closed, vec![child_scope, ScopeId(0)]);
        // Nothing publishes into a torn-down run.
        let operation = OperationId(1);
        assert_eq!(
            scheduler.publish_completion(record(&scheduler, root, operation)),
            Err(SchedulerFault::CrossRunCompletion)
        );
    }

    #[test]
    fn metadata_accounting_is_exact_and_charged_per_record() {
        let scheduler = scheduler();
        let baseline = scheduler.metadata_bytes();
        assert_eq!(baseline, SCOPE_RECORD_BYTES + TASK_RECORD_BYTES);
    }
}
