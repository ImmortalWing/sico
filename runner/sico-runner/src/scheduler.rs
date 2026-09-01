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
/// Operands of one select/race resolution.
pub const MAX_SELECT_OPERANDS: usize = 256;
/// Channels per run.
pub const MAX_CHANNELS: usize = 1_024;
/// Buffered items per channel (ADR-0010 hard limits).
pub const MAX_CHANNEL_ITEMS: usize = 1_024;
/// Per-channel byte budget ceiling.
pub const MAX_CHANNEL_BYTES: usize = 16 << 20;

/// Accounted sizes for the metadata budget. With every count capped at
/// 1,024 the budget is structurally unreachable (hundreds of KiB at most);
/// it is enforced anyway so a future larger record cannot grow silently.
const TASK_RECORD_BYTES: usize = 128;
const SCOPE_RECORD_BYTES: usize = 96;
const READY_ENTRY_BYTES: usize = 16;
const OPERATION_RECORD_BYTES: usize = 96;
const CHANNEL_RECORD_BYTES: usize = 128;

/// Run-local task identity; monotonically increasing within one run.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TaskId(pub u32);

/// Run-local task-scope identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScopeId(pub u32);

/// Run-local Host-operation registration identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationId(pub u64);

/// Run-local channel identity; monotonically increasing within one run.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChannelId(pub u32);

/// How a channel was closed (M11 STEP-0107): a clean owner close drains
/// buffered items before answering `closed`; a failure close (owner
/// cancelled or failed) propagates immediately to every waiter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseKind {
    Closed,
    Failed,
}

/// Result of a channel send (backpressure is suspension, never spinning).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SendOutcome {
    /// Handed directly to a waiting receiver.
    Delivered,
    /// Accepted into the bounded buffer.
    Buffered,
    /// Buffer full (or rendezvous without a receiver): the producer is
    /// suspended in the FIFO send-waiter queue until a slot frees.
    Blocked,
}

/// Result of a channel receive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecvOutcome {
    /// One buffered or handed-off item: `(sender, payload bytes)`.
    Item { sender: TaskId, bytes: usize },
    /// Empty open channel: the consumer is suspended in the FIFO
    /// recv-waiter queue until a send or close arrives.
    Blocked,
    /// The channel is closed and drained (or failure-closed).
    Closed(CloseKind),
}

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

/// The single result of one select/race resolution (M11 STEP-0106): the
/// canonical-first ready operand, its consumed completion record when the
/// readiness came from the ingress, and the deterministically cancelled
/// losers in the order they were committed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectOutcome {
    pub winner: TaskId,
    pub record: Option<CompletionRecord>,
    pub losers: Vec<TaskId>,
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
    SelectOperandsExceeded(usize),
    NoSelectOperands,
    NoReadyOperand,
    Deadlock(Vec<TaskId>),
    ChannelTableFull,
    ChannelItemsExceeded(usize),
    ChannelBytesExceeded,
    UnknownChannel(ChannelId),
    ChannelClosed(ChannelId),
    NotChannelOwner {
        channel: ChannelId,
        task: TaskId,
    },
    ChannelWaitConflict(TaskId),
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
            Self::SelectOperandsExceeded(count) => {
                write!(formatter, "select/race operands exceeded (256): {count}")
            }
            Self::NoSelectOperands => {
                formatter.write_str("select/race requires at least one operand")
            }
            Self::NoReadyOperand => {
                formatter.write_str("select/race has no ready operand this turn")
            }
            Self::Deadlock(tasks) => write!(
                formatter,
                "deadlock: {} suspended tasks with no runnable work or pending host operations",
                tasks.len()
            ),
            Self::ChannelTableFull => formatter.write_str("channel table full (1024 channels)"),
            Self::ChannelItemsExceeded(count) => {
                write!(formatter, "channel item capacity exceeded (1024): {count}")
            }
            Self::ChannelBytesExceeded => formatter.write_str("channel byte budget exceeded"),
            Self::UnknownChannel(channel) => write!(formatter, "unknown channel {}", channel.0),
            Self::ChannelClosed(channel) => write!(formatter, "channel {} is closed", channel.0),
            Self::NotChannelOwner { channel, task } => write!(
                formatter,
                "task {} does not own channel {}",
                task.0, channel.0
            ),
            Self::ChannelWaitConflict(task) => {
                write!(formatter, "task {} is already waiting on a channel", task.0)
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

/// One bounded channel (M11 STEP-0107): items carry `(sender, payload
/// bytes)` — payload contents stay in task memory until a suspension
/// profile exists; this table is the accounting/synchronization engine.
#[derive(Debug)]
struct Channel {
    owner: TaskId,
    item_capacity: usize,
    byte_budget: usize,
    items: VecDeque<(TaskId, usize)>,
    bytes: usize,
    closed: Option<CloseKind>,
    send_waiters: VecDeque<(TaskId, usize)>,
    recv_waiters: VecDeque<TaskId>,
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
    channels: BTreeMap<ChannelId, Channel>,
    next_channel: u32,
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
            channels: BTreeMap::new(),
            next_channel: 0,
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

    /// Cancels a task and every descendant in deterministic reverse
    /// ownership order (deepest scope, then highest task id), committing
    /// each non-terminal member's single `cancelled` state and abandoning
    /// its pending Host operations so a late worker record is stale.
    /// Cancellation is idempotent on already-terminal members and never
    /// travels upward (ADR-0010 §6.3). Returns the tasks this call
    /// actually committed, in commit order.
    pub fn cancel_task(&mut self, task: TaskId) -> Result<Vec<TaskId>, SchedulerFault> {
        if !self.tasks.contains_key(&task) {
            return Err(SchedulerFault::UnknownTask(task));
        }
        let mut members = vec![task];
        let mut index = 0;
        while index < members.len() {
            let current = members[index];
            for (child, record) in &self.tasks {
                if record.parent == Some(current) && !members.contains(child) {
                    members.push(*child);
                }
            }
            index += 1;
        }
        Ok(self.cancel_members(members))
    }

    /// Cancels every task in a scope subtree (the scope itself stays open
    /// until the usual close/teardown discipline).
    pub fn cancel_scope(&mut self, scope: ScopeId) -> Result<Vec<TaskId>, SchedulerFault> {
        if !self.scopes.contains_key(&scope) {
            return Err(SchedulerFault::UnknownScope(scope));
        }
        let mut scopes = vec![scope];
        let mut index = 0;
        while index < scopes.len() {
            let current = scopes[index];
            for (child, record) in &self.scopes {
                if record.parent == Some(current) && !scopes.contains(child) {
                    scopes.push(*child);
                }
            }
            index += 1;
        }
        let members: Vec<_> = self
            .tasks
            .iter()
            .filter(|(_, record)| scopes.contains(&record.scope))
            .map(|(task, _)| *task)
            .collect();
        Ok(self.cancel_members(members))
    }

    /// Cancels the root task (and therefore the whole tree); the run-level
    /// cancellation path shares the same entry as any descendant.
    pub fn cancel_root(&mut self) -> Result<Vec<TaskId>, SchedulerFault> {
        self.cancel_task(self.root_task())
    }

    /// Resolves one select/race over task operands (ADR-0010 §6.3): the
    /// winner is the canonical-first ready operand — cancelled operands
    /// first, then completion-ready operands by registration order, ties
    /// broken by task id, never wall-clock. Every other non-terminal
    /// operand is cancelled through the tree and its pending Host
    /// operations abandoned, so a loser cannot emit a second terminal
    /// state or retain Host work. The winner's queued completion record,
    /// when present, is consumed.
    pub fn select(&mut self, operands: &[TaskId]) -> Result<SelectOutcome, SchedulerFault> {
        if operands.is_empty() {
            return Err(SchedulerFault::NoSelectOperands);
        }
        if operands.len() > MAX_SELECT_OPERANDS {
            return Err(SchedulerFault::SelectOperandsExceeded(operands.len()));
        }
        let mut unique = operands.to_vec();
        unique.sort_unstable();
        unique.dedup();
        for task in &unique {
            if !self.tasks.contains_key(task) {
                return Err(SchedulerFault::UnknownTask(*task));
            }
        }
        let winner = unique
            .iter()
            .filter_map(|task| self.select_ready_key(task).map(|key| (key, *task)))
            .min_by_key(|(key, _)| *key)
            .map(|(_, task)| task)
            .ok_or(SchedulerFault::NoReadyOperand)?;
        let record = self
            .completions
            .iter()
            .position(|record| record.task == winner)
            .map(|position| self.completions.remove(position));
        let losers: Vec<_> = unique
            .iter()
            .copied()
            .filter(|task| *task != winner)
            .collect();
        let losers = self.cancel_members(losers);
        Ok(SelectOutcome {
            winner,
            record,
            losers,
        })
    }

    /// Proves the absorbing state: no runnable task, no pending Host
    /// operation, yet at least one task is suspended forever. Returns the
    /// blocked tasks as a typed fault instead of hanging.
    pub fn detect_deadlock(&self) -> Result<(), SchedulerFault> {
        let blocked: Vec<_> = self
            .tasks
            .iter()
            .filter(|(_, record)| record.state == TaskState::Suspended)
            .map(|(task, _)| *task)
            .collect();
        if blocked.is_empty() {
            return Ok(());
        }
        let any_runnable = self
            .tasks
            .values()
            .any(|record| record.state == TaskState::Runnable);
        let any_pending_operation = self.operations.values().any(|record| !record.terminal);
        if any_runnable || any_pending_operation {
            return Ok(());
        }
        Err(SchedulerFault::Deadlock(blocked))
    }

    /// Opens a bounded channel owned by a live task (M11 STEP-0107).
    /// `item_capacity` 0 is a rendezvous channel; `byte_budget` bounds
    /// buffered payload bytes. Channel records charge the metadata budget.
    pub fn open_channel(
        &mut self,
        owner: TaskId,
        item_capacity: usize,
        byte_budget: usize,
    ) -> Result<ChannelId, SchedulerFault> {
        self.require_live_task(owner)?;
        if item_capacity > MAX_CHANNEL_ITEMS {
            return Err(SchedulerFault::ChannelItemsExceeded(item_capacity));
        }
        if byte_budget > MAX_CHANNEL_BYTES {
            return Err(SchedulerFault::ChannelBytesExceeded);
        }
        if self.channels.len() >= MAX_CHANNELS {
            return Err(SchedulerFault::ChannelTableFull);
        }
        self.charge_metadata(CHANNEL_RECORD_BYTES)?;
        let channel = ChannelId(self.next_channel);
        self.next_channel += 1;
        self.channels.insert(
            channel,
            Channel {
                owner,
                item_capacity,
                byte_budget,
                items: VecDeque::new(),
                bytes: 0,
                closed: None,
                send_waiters: VecDeque::new(),
                recv_waiters: VecDeque::new(),
            },
        );
        Ok(channel)
    }

    /// Sends one item carrying `bytes` of payload accounting. A waiting
    /// receiver gets a direct handoff; otherwise the item buffers within
    /// the item/byte budgets; otherwise the producer suspends in the FIFO
    /// send-waiter queue (backpressure, never spinning).
    pub fn send(
        &mut self,
        task: TaskId,
        channel: ChannelId,
        bytes: usize,
    ) -> Result<SendOutcome, SchedulerFault> {
        self.require_live_task(task)?;
        {
            let record = self
                .channels
                .get(&channel)
                .ok_or(SchedulerFault::UnknownChannel(channel))?;
            if record.closed.is_some() {
                return Err(SchedulerFault::ChannelClosed(channel));
            }
            if bytes > record.byte_budget {
                return Err(SchedulerFault::ChannelBytesExceeded);
            }
        }
        if let Some(consumer) = self
            .channels
            .get_mut(&channel)
            .expect("channel checked")
            .recv_waiters
            .pop_front()
        {
            self.make_runnable(consumer)?;
            return Ok(SendOutcome::Delivered);
        }
        {
            let record = self.channels.get_mut(&channel).expect("channel checked");
            if record.items.len() < record.item_capacity
                && record.bytes + bytes <= record.byte_budget
            {
                record.items.push_back((task, bytes));
                record.bytes += bytes;
                return Ok(SendOutcome::Buffered);
            }
        }
        self.check_wait_conflict(task)?;
        self.suspend(task)?;
        self.channels
            .get_mut(&channel)
            .expect("channel checked")
            .send_waiters
            .push_back((task, bytes));
        Ok(SendOutcome::Blocked)
    }

    /// Receives one item: buffered FIFO first (promoting the oldest
    /// blocked producer into the freed slot), then a rendezvous handoff
    /// from a blocked sender, then `closed` once drained; otherwise the
    /// consumer suspends in the FIFO recv-waiter queue.
    pub fn recv(
        &mut self,
        task: TaskId,
        channel: ChannelId,
    ) -> Result<RecvOutcome, SchedulerFault> {
        self.require_live_task(task)?;
        if !self.channels.contains_key(&channel) {
            return Err(SchedulerFault::UnknownChannel(channel));
        }
        let buffered = self
            .channels
            .get_mut(&channel)
            .expect("channel checked")
            .items
            .pop_front();
        if let Some((sender, bytes)) = buffered {
            self.channels
                .get_mut(&channel)
                .expect("channel checked")
                .bytes -= bytes;
            self.promote_senders(channel);
            return Ok(RecvOutcome::Item { sender, bytes });
        }
        let handoff = self
            .channels
            .get_mut(&channel)
            .expect("channel checked")
            .send_waiters
            .pop_front();
        if let Some((sender, bytes)) = handoff {
            self.make_runnable(sender)?;
            return Ok(RecvOutcome::Item { sender, bytes });
        }
        if let Some(kind) = self.channels.get(&channel).expect("channel checked").closed {
            return Ok(RecvOutcome::Closed(kind));
        }
        self.check_wait_conflict(task)?;
        self.suspend(task)?;
        self.channels
            .get_mut(&channel)
            .expect("channel checked")
            .recv_waiters
            .push_back(task);
        Ok(RecvOutcome::Blocked)
    }

    /// Owner-only close. Buffered items keep draining; current waiters wake
    /// immediately (receivers observe `closed` once drained, senders get a
    /// typed refusal); double close is a typed fault.
    pub fn close_channel(
        &mut self,
        task: TaskId,
        channel: ChannelId,
        kind: CloseKind,
    ) -> Result<(), SchedulerFault> {
        {
            let record = self
                .channels
                .get(&channel)
                .ok_or(SchedulerFault::UnknownChannel(channel))?;
            if record.owner != task {
                return Err(SchedulerFault::NotChannelOwner { channel, task });
            }
            if record.closed.is_some() {
                return Err(SchedulerFault::ChannelClosed(channel));
            }
        }
        let record = self.channels.get_mut(&channel).expect("channel checked");
        record.closed = Some(kind);
        let send_waiters: Vec<_> = record
            .send_waiters
            .drain(..)
            .map(|(task, _)| task)
            .collect();
        let recv_waiters: Vec<_> = record.recv_waiters.drain(..).collect();
        for waiter in send_waiters.into_iter().chain(recv_waiters) {
            if self.task_state(waiter) == Some(TaskState::Suspended) {
                self.make_runnable(waiter)?;
            }
        }
        Ok(())
    }

    /// Transfers channel ownership (affine move); the old owner's later
    /// close is a typed `NotChannelOwner` failure.
    pub fn move_channel(
        &mut self,
        task: TaskId,
        channel: ChannelId,
        new_owner: TaskId,
    ) -> Result<(), SchedulerFault> {
        self.require_live_task(new_owner)?;
        let record = self
            .channels
            .get_mut(&channel)
            .ok_or(SchedulerFault::UnknownChannel(channel))?;
        if record.owner != task {
            return Err(SchedulerFault::NotChannelOwner { channel, task });
        }
        record.owner = new_owner;
        Ok(())
    }

    /// Promotes blocked senders into freed buffer slots, oldest first.
    fn promote_senders(&mut self, channel: ChannelId) {
        let mut wake = Vec::new();
        {
            let record = self.channels.get_mut(&channel).expect("channel checked");
            while let Some(&(sender, bytes)) = record.send_waiters.front() {
                if record.items.len() >= record.item_capacity
                    || record.bytes + bytes > record.byte_budget
                {
                    break;
                }
                record.send_waiters.pop_front();
                record.items.push_back((sender, bytes));
                record.bytes += bytes;
                wake.push(sender);
            }
        }
        for sender in wake {
            if self.task_state(sender) == Some(TaskState::Suspended) {
                self.make_runnable(sender).expect("waiter was suspended");
            }
        }
    }

    /// A task may wait on at most one channel at a time.
    fn check_wait_conflict(&self, task: TaskId) -> Result<(), SchedulerFault> {
        let waiting = self.channels.values().any(|record| {
            record.recv_waiters.contains(&task)
                || record
                    .send_waiters
                    .iter()
                    .any(|(waiter, _)| *waiter == task)
        });
        if waiting {
            return Err(SchedulerFault::ChannelWaitConflict(task));
        }
        Ok(())
    }

    fn require_live_task(&self, task: TaskId) -> Result<(), SchedulerFault> {
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
        // Idempotent cleanup path (ADR-0010): any channel still open at
        // teardown closes in reverse registration order; every task is
        // already terminal, so no waiter can exist to wake.
        let mut open_channels: Vec<_> = self
            .channels
            .iter()
            .filter(|(_, channel)| channel.closed.is_none())
            .map(|(id, _)| *id)
            .collect();
        open_channels.sort_by_key(|id| Reverse(id.0));
        for id in open_channels {
            let channel = self.channels.get_mut(&id).expect("channel listed");
            channel.closed = Some(CloseKind::Closed);
            channel.send_waiters.clear();
            channel.recv_waiters.clear();
        }
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

    /// Commits `cancelled` to every non-terminal member in deterministic
    /// reverse ownership order (deepest scope, then highest task id) and
    /// abandons each member's pending Host operations. Terminal members
    /// keep their committed state — one terminal per task, always.
    fn cancel_members(&mut self, mut members: Vec<TaskId>) -> Vec<TaskId> {
        members.sort_by_key(|task| {
            let depth = self
                .tasks
                .get(task)
                .and_then(|record| self.scopes.get(&record.scope))
                .map_or(0, |scope| scope.depth);
            (Reverse(depth), Reverse(task.0))
        });
        let mut cancelled = Vec::new();
        for member in members {
            if let Some(record) = self.tasks.get_mut(&member)
                && !record.state.is_terminal()
            {
                record.state = TaskState::Cancelled;
                cancelled.push(member);
            }
            for operation in self.operations.values_mut() {
                if operation.task == member && !operation.terminal {
                    operation.terminal = true;
                }
            }
            // A cancelled task leaves every channel wait queue, and the
            // channels it owns failure-close so their consumers can never
            // hang on a dead producer (ADR-0010 reverse-order cleanup).
            for channel in self.channels.values_mut() {
                channel.send_waiters.retain(|(waiter, _)| *waiter != member);
                channel.recv_waiters.retain(|waiter| *waiter != member);
            }
            let owned: Vec<_> = self
                .channels
                .iter()
                .filter(|(_, channel)| channel.owner == member && channel.closed.is_none())
                .map(|(id, _)| *id)
                .collect();
            for channel in owned {
                let record = self.channels.get_mut(&channel).expect("channel listed");
                record.closed = Some(CloseKind::Failed);
                let send_waiters: Vec<_> = record
                    .send_waiters
                    .drain(..)
                    .map(|(task, _)| task)
                    .collect();
                let recv_waiters: Vec<_> = record.recv_waiters.drain(..).collect();
                for waiter in send_waiters.into_iter().chain(recv_waiters) {
                    if self.task_state(waiter) == Some(TaskState::Suspended) {
                        self.make_runnable(waiter).expect("waiter was suspended");
                    }
                }
            }
        }
        cancelled
    }

    /// Canonical readiness key of a select operand, when ready: cancelled
    /// tasks sort before everything (ADR-0010 §6.3 class order), then
    /// operands with a queued completion by their operation's
    /// registration-time class and order, then other terminal tasks; ties
    /// break on task id.
    fn select_ready_key(&self, task: &TaskId) -> Option<(ReadinessClass, u64, TaskId)> {
        let record = self.tasks.get(task)?;
        let earliest_operation_order = || {
            self.operations
                .iter()
                .filter(|(_, operation)| operation.task == *task)
                .filter_map(|(operation, _)| self.operation_order.get(operation).copied())
                .min()
                .unwrap_or(u64::MAX)
        };
        if record.state == TaskState::Cancelled {
            return Some((
                ReadinessClass::Cancellation,
                earliest_operation_order(),
                *task,
            ));
        }
        if let Some(completion) = self
            .completions
            .iter()
            .find(|completion| completion.task == *task)
        {
            let operation = self.operations.get(&completion.operation)?;
            let order = self
                .operation_order
                .get(&completion.operation)
                .copied()
                .unwrap_or(u64::MAX);
            return Some((operation.class, order, *task));
        }
        if record.state.is_terminal() {
            return Some((ReadinessClass::HostCompletion, u64::MAX, *task));
        }
        None
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

    // ---- STEP-0106: cancellation tree, select/race, deadlock ----

    /// Builds a root → a → {b, c} tree with one pending operation on each
    /// of b and c, all inside one child scope.
    fn tree_fixture() -> (
        SchedulerCore,
        TaskId,
        TaskId,
        TaskId,
        OperationId,
        OperationId,
    ) {
        let mut scheduler = scheduler();
        let root = scheduler.root_task();
        scheduler.make_runnable(root).unwrap();
        let _ = scheduler.next_ready();
        let scope = scheduler.open_scope(ScopeId(0)).unwrap();
        let a = scheduler.spawn_task(scope, root).unwrap();
        let b = scheduler.spawn_task(scope, a).unwrap();
        let c = scheduler.spawn_task(scope, a).unwrap();
        let op_b = scheduler
            .register_operation(b, ReadinessClass::HostCompletion)
            .unwrap();
        let op_c = scheduler
            .register_operation(c, ReadinessClass::Timer)
            .unwrap();
        (scheduler, a, b, c, op_b, op_c)
    }

    #[test]
    fn cancel_task_propagates_down_in_reverse_order_and_abandons_host_work() {
        let (mut scheduler, a, b, c, op_b, op_c) = tree_fixture();
        let cancelled = scheduler.cancel_task(a).unwrap();
        // Reverse ownership order: same depth, highest id first.
        assert_eq!(cancelled, vec![c, b, a]);
        for task in [a, b, c] {
            assert_eq!(scheduler.task_state(task), Some(TaskState::Cancelled));
        }
        // Pending Host work is abandoned: late worker records are stale.
        assert_eq!(
            scheduler.publish_completion(record(&scheduler, b, op_b)),
            Err(SchedulerFault::StaleCompletion(op_b))
        );
        assert_eq!(
            scheduler.publish_completion(record(&scheduler, c, op_c)),
            Err(SchedulerFault::StaleCompletion(op_c))
        );
        // The parent is untouched: cancellation never travels upward.
        assert_eq!(
            scheduler.task_state(scheduler.root_task()),
            Some(TaskState::Runnable)
        );
        // Losers cannot emit a second terminal state.
        assert!(matches!(
            scheduler.commit_terminal(b, TerminalKind::Succeeded),
            Err(SchedulerFault::IllegalTransition { .. })
        ));
        // Cancellation is idempotent on already-terminal subtrees.
        assert_eq!(scheduler.cancel_task(a).unwrap(), Vec::<TaskId>::new());
        assert_eq!(
            scheduler.cancel_task(TaskId(999)),
            Err(SchedulerFault::UnknownTask(TaskId(999)))
        );
    }

    #[test]
    fn cancel_scope_covers_the_whole_subtree() {
        let (mut scheduler, a, b, c, _, _) = tree_fixture();
        let cancelled = scheduler.cancel_scope(ScopeId(1)).unwrap();
        assert_eq!(cancelled, vec![c, b, a]);
        assert_eq!(
            scheduler.cancel_scope(ScopeId(999)),
            Err(SchedulerFault::UnknownScope(ScopeId(999)))
        );
    }

    #[test]
    fn completion_vs_cancel_and_timeout_vs_cancel_have_one_result() {
        // Completion vs cancel, same turn: the cancelled operand is
        // observed first even though the host completion was delivered.
        let (mut scheduler, _a, b, c, op_b, _) = tree_fixture();
        scheduler
            .publish_completion(record(&scheduler, b, op_b))
            .unwrap();
        scheduler.cancel_task(c).unwrap();
        let outcome = scheduler.select(&[b, c]).unwrap();
        assert_eq!(outcome.winner, c);
        assert_eq!(outcome.record, None);
        assert_eq!(outcome.losers, vec![b]);
        // The loser is cancelled with its Host work abandoned; its record,
        // delivered before the cancellation, still drains exactly once.
        assert_eq!(scheduler.task_state(b), Some(TaskState::Cancelled));
        assert_eq!(
            scheduler.publish_completion(record(&scheduler, b, op_b)),
            Err(SchedulerFault::DuplicateCompletion(op_b))
        );

        // Timeout vs cancel, same turn: Cancellation < Timer in the
        // canonical class order, so the cancelled operand wins.
        let (mut scheduler, _a, b, c, _, op_c) = tree_fixture();
        scheduler
            .publish_completion(record(&scheduler, c, op_c))
            .unwrap();
        scheduler.cancel_task(b).unwrap();
        let outcome = scheduler.select(&[b, c]).unwrap();
        assert_eq!(outcome.winner, b);
        assert_eq!(outcome.losers, vec![c]);
        assert_eq!(scheduler.task_state(c), Some(TaskState::Cancelled));
    }

    #[test]
    fn simultaneous_ready_uses_registration_order_then_task_id() {
        let mut scheduler = scheduler();
        let root = scheduler.root_task();
        scheduler.make_runnable(root).unwrap();
        let _ = scheduler.next_ready();
        let scope = scheduler.open_scope(ScopeId(0)).unwrap();
        let mut operands = Vec::new();
        let mut operations = Vec::new();
        for _ in 0..4 {
            let task = scheduler.spawn_task(scope, root).unwrap();
            let operation = scheduler
                .register_operation(task, ReadinessClass::HostCompletion)
                .unwrap();
            operands.push(task);
            operations.push(operation);
        }
        // Publish out of order: registration order, not publish order, wins.
        for index in [3, 1, 2, 0] {
            scheduler
                .publish_completion(record(&scheduler, operands[index], operations[index]))
                .unwrap();
        }
        let outcome = scheduler.select(&operands).unwrap();
        assert_eq!(outcome.winner, operands[0]);
        assert_eq!(
            outcome.record.as_ref().map(|record| record.operation),
            Some(operations[0])
        );
        // Losers commit in reverse ownership order (same depth, highest
        // id first).
        let expected_losers: Vec<_> = operands[1..].iter().rev().copied().collect();
        assert_eq!(outcome.losers, expected_losers);
        // The consumed winner record is gone; the losers' queued records
        // still drain in canonical registration order.
        let drained: Vec<_> = scheduler
            .drain_turn()
            .iter()
            .map(|record| record.operation)
            .collect();
        assert_eq!(drained, operations[1..].to_vec());
    }

    #[test]
    fn parent_vs_child_failure_keeps_one_terminal_each() {
        let (mut scheduler, a, b, c, _, _) = tree_fixture();
        // Child fails first; parent cancellation afterwards must not
        // rewrite the child's committed failure.
        scheduler.commit_terminal(b, TerminalKind::Failed).unwrap();
        let cancelled = scheduler.cancel_task(a).unwrap();
        assert_eq!(cancelled, vec![c, a]);
        assert_eq!(scheduler.task_state(b), Some(TaskState::Failed));
        // Child failure alone cannot widen upward either.
        assert_eq!(
            scheduler.task_state(scheduler.root_task()),
            Some(TaskState::Runnable)
        );
    }

    #[test]
    fn select_operand_bounds_are_typed() {
        let mut scheduler = scheduler();
        let root = scheduler.root_task();
        scheduler.make_runnable(root).unwrap();
        let _ = scheduler.next_ready();
        let scope = scheduler.open_scope(ScopeId(0)).unwrap();
        let mut operands = Vec::new();
        for _ in 0..MAX_SELECT_OPERANDS {
            operands.push(scheduler.spawn_task(scope, root).unwrap());
        }
        // Nothing ready: typed would-block, not a hang.
        assert_eq!(
            scheduler.select(&operands),
            Err(SchedulerFault::NoReadyOperand)
        );
        scheduler
            .commit_terminal(operands[10], TerminalKind::Succeeded)
            .unwrap();
        assert_eq!(scheduler.select(&operands).unwrap().winner, operands[10]);
        // 257 operands and the empty select are typed refusals.
        let mut too_many = operands.clone();
        too_many.push(scheduler.spawn_task(scope, root).unwrap());
        assert_eq!(
            scheduler.select(&too_many),
            Err(SchedulerFault::SelectOperandsExceeded(257))
        );
        assert_eq!(scheduler.select(&[]), Err(SchedulerFault::NoSelectOperands));
        assert_eq!(
            scheduler.select(&[TaskId(999)]),
            Err(SchedulerFault::UnknownTask(TaskId(999)))
        );
    }

    #[test]
    fn deadlock_is_provable_and_typed() {
        let (mut scheduler, a, b, c, _, _) = tree_fixture();
        let root = scheduler.root_task();
        for task in [a, b, c] {
            scheduler.make_runnable(task).unwrap();
            let _ = scheduler.next_ready();
            scheduler.suspend(task).unwrap();
        }
        // Pending Host operations can still wake the tasks: no deadlock.
        // The root task is still runnable after its pop, which also
        // disproves deadlock.
        assert_eq!(scheduler.detect_deadlock(), Ok(()));
        scheduler.suspend(root).unwrap();
        // Abandon every operation: the suspension can never resolve.
        let operations: Vec<_> = (1..=2).map(OperationId).collect();
        for operation in operations {
            scheduler.abandon_operation(operation).unwrap();
        }
        let Err(SchedulerFault::Deadlock(blocked)) = scheduler.detect_deadlock() else {
            panic!("expected deadlock");
        };
        assert_eq!(blocked, vec![root, a, b, c]);
        // Cancelling the blocked tasks resolves the absorbing state.
        scheduler.cancel_task(a).unwrap();
        scheduler.cancel_task(root).unwrap();
        assert_eq!(scheduler.detect_deadlock(), Ok(()));
    }

    #[test]
    fn nested_cancellation_of_a_full_task_chain_stays_within_bounds() {
        let mut scheduler = scheduler();
        let root = scheduler.root_task();
        scheduler.make_runnable(root).unwrap();
        let _ = scheduler.next_ready();
        // A 1,024-deep task parent chain in one scope (parent chains are
        // not scope nesting; the scope cap admits root + 1,023 children).
        let mut parent = root;
        for _ in 1..MAX_LIVE_TASKS {
            parent = scheduler.spawn_task(ScopeId(0), parent).unwrap();
        }
        let started = std::time::Instant::now();
        let cancelled = scheduler.cancel_task(root).unwrap();
        let elapsed = started.elapsed();
        assert_eq!(cancelled.len(), MAX_LIVE_TASKS);
        assert_eq!(scheduler.live_tasks(), 0);
        // Deepest scope ties break on highest id first: the chain head
        // (root) commits last.
        assert_eq!(cancelled.last(), Some(&root));
        assert_eq!(scheduler.teardown().unwrap(), vec![ScopeId(0)]);
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "1,024-chain cancellation took {elapsed:?}"
        );
        println!("CHAIN_CANCEL_1024 wall_us={}", elapsed.as_micros());
    }

    // ---- STEP-0107: bounded channels, streams and backpressure ----

    /// Spawns a task and drives it to the running point (runnable, popped).
    fn running_task(scheduler: &mut SchedulerCore, parent: TaskId) -> TaskId {
        let task = scheduler.spawn_task(ScopeId(0), parent).unwrap();
        scheduler.make_runnable(task).unwrap();
        assert_eq!(scheduler.next_ready(), Some(task));
        task
    }

    /// Root runnable with the run started.
    fn channel_scheduler() -> (SchedulerCore, TaskId) {
        let mut scheduler = scheduler();
        let root = scheduler.root_task();
        scheduler.make_runnable(root).unwrap();
        assert_eq!(scheduler.next_ready(), Some(root));
        (scheduler, root)
    }

    #[test]
    fn channel_capacity_zero_is_a_rendezvous() {
        let (mut scheduler, root) = channel_scheduler();
        let producer = running_task(&mut scheduler, root);
        let consumer = running_task(&mut scheduler, root);
        let channel = scheduler.open_channel(root, 0, 1_024).unwrap();

        // Sender first: blocks until a receiver arrives.
        assert_eq!(
            scheduler.send(producer, channel, 16),
            Ok(SendOutcome::Blocked)
        );
        assert_eq!(scheduler.task_state(producer), Some(TaskState::Suspended));
        // Nothing spins: the blocked producer is not in the ready queue.
        assert_eq!(scheduler.next_ready(), None);
        assert_eq!(
            scheduler.recv(consumer, channel),
            Ok(RecvOutcome::Item {
                sender: producer,
                bytes: 16
            })
        );
        assert_eq!(scheduler.task_state(producer), Some(TaskState::Runnable));

        // Receiver first: blocks until a sender arrives. The woken
        // producer is still queued from the handoff; pop it first.
        assert_eq!(scheduler.next_ready(), Some(producer));
        assert_eq!(scheduler.recv(consumer, channel), Ok(RecvOutcome::Blocked));
        assert_eq!(scheduler.next_ready(), None);
        assert_eq!(
            scheduler.send(producer, channel, 8),
            Ok(SendOutcome::Delivered)
        );
        assert_eq!(scheduler.task_state(consumer), Some(TaskState::Runnable));
    }

    #[test]
    fn channel_capacity_one_buffers_then_blocks() {
        let (mut scheduler, root) = channel_scheduler();
        let producer = running_task(&mut scheduler, root);
        let consumer = running_task(&mut scheduler, root);
        let channel = scheduler.open_channel(root, 1, 1_024).unwrap();

        assert_eq!(
            scheduler.send(producer, channel, 4),
            Ok(SendOutcome::Buffered)
        );
        assert_eq!(
            scheduler.send(producer, channel, 4),
            Ok(SendOutcome::Blocked)
        );
        // A recv frees the slot and promotes the blocked producer FIFO.
        assert!(matches!(
            scheduler.recv(consumer, channel),
            Ok(RecvOutcome::Item { .. })
        ));
        assert_eq!(scheduler.task_state(producer), Some(TaskState::Runnable));
        assert!(matches!(
            scheduler.recv(consumer, channel),
            Ok(RecvOutcome::Item { .. })
        ));
    }

    #[test]
    fn channel_limits_are_typed_and_backpressure_blocks() {
        let (mut scheduler, root) = channel_scheduler();
        assert_eq!(
            scheduler.open_channel(root, MAX_CHANNEL_ITEMS + 1, 1_024),
            Err(SchedulerFault::ChannelItemsExceeded(MAX_CHANNEL_ITEMS + 1))
        );
        assert_eq!(
            scheduler.open_channel(root, 1, MAX_CHANNEL_BYTES + 1),
            Err(SchedulerFault::ChannelBytesExceeded)
        );
        // Fill a max-capacity channel: the max+1th send blocks (typed
        // backpressure), it does not error and does not grow the queue.
        let channel = scheduler
            .open_channel(root, MAX_CHANNEL_ITEMS, MAX_CHANNEL_BYTES)
            .unwrap();
        for _ in 0..MAX_CHANNEL_ITEMS {
            assert_eq!(scheduler.send(root, channel, 1), Ok(SendOutcome::Buffered));
        }
        let extra = running_task(&mut scheduler, root);
        assert_eq!(scheduler.send(extra, channel, 1), Ok(SendOutcome::Blocked));
        // Byte budget: a single item larger than the budget is a typed
        // fault; an item that merely overflows the remaining budget blocks.
        let tight = scheduler.open_channel(root, 8, 100).unwrap();
        assert_eq!(
            scheduler.send(root, tight, 101),
            Err(SchedulerFault::ChannelBytesExceeded)
        );
        assert_eq!(scheduler.send(root, tight, 60), Ok(SendOutcome::Buffered));
        let second = running_task(&mut scheduler, root);
        assert_eq!(scheduler.send(second, tight, 50), Ok(SendOutcome::Blocked));
        // Channel table cap.
        let mut last = None;
        for _ in scheduler.channels.len()..MAX_CHANNELS {
            last = Some(scheduler.open_channel(root, 0, 0).unwrap());
        }
        assert!(last.is_some());
        assert_eq!(
            scheduler.open_channel(root, 0, 0),
            Err(SchedulerFault::ChannelTableFull)
        );
    }

    #[test]
    fn slow_consumer_gets_fifo_fairness_without_spinning() {
        let (mut scheduler, root) = channel_scheduler();
        let consumer = running_task(&mut scheduler, root);
        let channel = scheduler.open_channel(root, 2, 1_024).unwrap();
        let producers: Vec<_> = (0..4).map(|_| running_task(&mut scheduler, root)).collect();
        assert_eq!(
            scheduler.send(producers[0], channel, 1),
            Ok(SendOutcome::Buffered)
        );
        assert_eq!(
            scheduler.send(producers[1], channel, 1),
            Ok(SendOutcome::Buffered)
        );
        assert_eq!(
            scheduler.send(producers[2], channel, 1),
            Ok(SendOutcome::Blocked)
        );
        assert_eq!(
            scheduler.send(producers[3], channel, 1),
            Ok(SendOutcome::Blocked)
        );
        // The ready queue holds nothing: backpressure parks producers.
        assert_eq!(scheduler.next_ready(), None);
        // Slow consumer drains one at a time; producers wake in arrival
        // order and items keep creation order.
        for round in 0..4 {
            let outcome = scheduler.recv(consumer, channel).unwrap();
            assert_eq!(
                outcome,
                RecvOutcome::Item {
                    sender: producers[round],
                    bytes: 1
                }
            );
            if round < 2 {
                let woken = producers[round + 2];
                assert_eq!(scheduler.task_state(woken), Some(TaskState::Runnable));
                assert_eq!(scheduler.next_ready(), Some(woken));
            }
        }
    }

    #[test]
    fn early_close_drains_then_reports_closed_and_refuses_sends() {
        let (mut scheduler, root) = channel_scheduler();
        let producer = running_task(&mut scheduler, root);
        let consumer = running_task(&mut scheduler, root);
        let channel = scheduler.open_channel(root, 4, 1_024).unwrap();
        scheduler.send(producer, channel, 1).unwrap();
        scheduler.send(producer, channel, 2).unwrap();
        scheduler
            .close_channel(root, channel, CloseKind::Closed)
            .unwrap();
        assert_eq!(
            scheduler.send(producer, channel, 3),
            Err(SchedulerFault::ChannelClosed(channel))
        );
        assert!(matches!(
            scheduler.recv(consumer, channel),
            Ok(RecvOutcome::Item { bytes: 1, .. })
        ));
        assert!(matches!(
            scheduler.recv(consumer, channel),
            Ok(RecvOutcome::Item { bytes: 2, .. })
        ));
        assert_eq!(
            scheduler.recv(consumer, channel),
            Ok(RecvOutcome::Closed(CloseKind::Closed))
        );
        // Double close and non-owner close are typed faults.
        assert_eq!(
            scheduler.close_channel(root, channel, CloseKind::Closed),
            Err(SchedulerFault::ChannelClosed(channel))
        );
        assert_eq!(
            scheduler.close_channel(producer, channel, CloseKind::Closed),
            Err(SchedulerFault::NotChannelOwner {
                channel,
                task: producer
            })
        );
    }

    #[test]
    fn close_wakes_waiters_with_the_close_kind() {
        let (mut scheduler, root) = channel_scheduler();
        // A blocked receiver on an empty rendezvous channel wakes at close
        // and observes `closed`.
        let consumer = running_task(&mut scheduler, root);
        let rendezvous = scheduler.open_channel(root, 0, 1_024).unwrap();
        assert_eq!(
            scheduler.recv(consumer, rendezvous),
            Ok(RecvOutcome::Blocked)
        );
        scheduler
            .close_channel(root, rendezvous, CloseKind::Closed)
            .unwrap();
        assert_eq!(scheduler.task_state(consumer), Some(TaskState::Runnable));
        assert_eq!(scheduler.next_ready(), Some(consumer));
        assert_eq!(
            scheduler.recv(consumer, rendezvous),
            Ok(RecvOutcome::Closed(CloseKind::Closed))
        );
        // A blocked sender on a full channel wakes at close and its retry
        // is a typed refusal.
        let producer = running_task(&mut scheduler, root);
        let full = scheduler.open_channel(root, 1, 1_024).unwrap();
        assert_eq!(scheduler.send(root, full, 1), Ok(SendOutcome::Buffered));
        assert_eq!(scheduler.send(producer, full, 1), Ok(SendOutcome::Blocked));
        scheduler
            .close_channel(root, full, CloseKind::Closed)
            .unwrap();
        assert_eq!(scheduler.task_state(producer), Some(TaskState::Runnable));
        assert_eq!(scheduler.next_ready(), Some(producer));
        assert_eq!(
            scheduler.send(producer, full, 1),
            Err(SchedulerFault::ChannelClosed(full))
        );
    }

    #[test]
    fn producer_failure_propagates_to_consumers() {
        let (mut scheduler, root) = channel_scheduler();
        let producer = running_task(&mut scheduler, root);
        let consumer = running_task(&mut scheduler, root);
        let channel = scheduler.open_channel(producer, 4, 1_024).unwrap();
        assert_eq!(scheduler.recv(consumer, channel), Ok(RecvOutcome::Blocked));
        // Cancelling the owner failure-closes its channels and wakes the
        // consumer instead of letting it hang.
        scheduler.cancel_task(producer).unwrap();
        assert_eq!(scheduler.task_state(consumer), Some(TaskState::Runnable));
        assert_eq!(
            scheduler.recv(consumer, channel),
            Ok(RecvOutcome::Closed(CloseKind::Failed))
        );
    }

    #[test]
    fn cancellation_removes_waiters_without_waking_them() {
        let (mut scheduler, root) = channel_scheduler();
        let consumer = running_task(&mut scheduler, root);
        let waiter = running_task(&mut scheduler, root);
        let channel = scheduler.open_channel(root, 0, 1_024).unwrap();
        assert_eq!(scheduler.recv(waiter, channel), Ok(RecvOutcome::Blocked));
        scheduler.cancel_task(waiter).unwrap();
        // The cancelled waiter is gone from the queue: a send now blocks
        // (no receiver), and closing does not try to wake a terminal task.
        let producer = running_task(&mut scheduler, root);
        assert_eq!(
            scheduler.send(producer, channel, 1),
            Ok(SendOutcome::Blocked)
        );
        scheduler
            .close_channel(root, channel, CloseKind::Closed)
            .unwrap();
        assert_eq!(scheduler.task_state(waiter), Some(TaskState::Cancelled));
        assert_eq!(scheduler.task_state(consumer), Some(TaskState::Runnable));
    }

    #[test]
    fn move_channel_transfers_ownership_affinely() {
        let (mut scheduler, root) = channel_scheduler();
        let new_owner = running_task(&mut scheduler, root);
        let channel = scheduler.open_channel(root, 1, 1_024).unwrap();
        scheduler.move_channel(root, channel, new_owner).unwrap();
        assert_eq!(
            scheduler.close_channel(root, channel, CloseKind::Closed),
            Err(SchedulerFault::NotChannelOwner {
                channel,
                task: root
            })
        );
        scheduler
            .close_channel(new_owner, channel, CloseKind::Closed)
            .unwrap();
        // Unknown channels are typed everywhere.
        assert_eq!(
            scheduler.send(root, ChannelId(999), 1),
            Err(SchedulerFault::UnknownChannel(ChannelId(999)))
        );
        assert_eq!(
            scheduler.recv(root, ChannelId(999)),
            Err(SchedulerFault::UnknownChannel(ChannelId(999)))
        );
    }

    #[test]
    fn channel_wait_conflict_and_terminal_refusals_are_typed() {
        let (mut scheduler, root) = channel_scheduler();
        let task = running_task(&mut scheduler, root);
        let first = scheduler.open_channel(root, 0, 1_024).unwrap();
        let second = scheduler.open_channel(root, 0, 1_024).unwrap();
        assert_eq!(scheduler.recv(task, first), Ok(RecvOutcome::Blocked));
        assert_eq!(
            scheduler.recv(task, second),
            Err(SchedulerFault::ChannelWaitConflict(task))
        );
        // Terminal tasks cannot send, recv or open channels.
        scheduler
            .commit_terminal(root, TerminalKind::Succeeded)
            .unwrap();
        assert!(matches!(
            scheduler.send(root, first, 1),
            Err(SchedulerFault::IllegalTransition { .. })
        ));
        assert!(matches!(
            scheduler.recv(root, first),
            Err(SchedulerFault::IllegalTransition { .. })
        ));
        assert!(matches!(
            scheduler.open_channel(root, 1, 1),
            Err(SchedulerFault::IllegalTransition { .. })
        ));
    }

    #[test]
    fn channel_deadlock_is_provable() {
        let (mut scheduler, root) = channel_scheduler();
        let first = running_task(&mut scheduler, root);
        let second = running_task(&mut scheduler, root);
        let channel_a = scheduler.open_channel(first, 0, 1_024).unwrap();
        let channel_b = scheduler.open_channel(second, 0, 1_024).unwrap();
        // Both producers block on empty rendezvous channels; the root
        // suspends; nothing can ever wake anyone.
        assert_eq!(
            scheduler.send(first, channel_a, 1),
            Ok(SendOutcome::Blocked)
        );
        assert_eq!(
            scheduler.send(second, channel_b, 1),
            Ok(SendOutcome::Blocked)
        );
        scheduler.suspend(root).unwrap();
        let Err(SchedulerFault::Deadlock(blocked)) = scheduler.detect_deadlock() else {
            panic!("expected channel deadlock");
        };
        assert_eq!(blocked, vec![root, first, second]);
        // Cancelling the subtree resolves it.
        scheduler.cancel_task(root).unwrap();
        assert_eq!(scheduler.detect_deadlock(), Ok(()));
    }
}
