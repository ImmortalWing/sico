use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    marker::PhantomData,
    rc::{Rc, Weak},
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Audit {
    pub events: Vec<String>,
    pub closes: u32,
    pub drops: u32,
    pub cancellations: u32,
    pub completions: u32,
    pub backpressure: u32,
    pub max_in_flight: usize,
}

pub type SharedAudit = Rc<RefCell<Audit>>;

pub fn new_audit() -> SharedAudit {
    Rc::new(RefCell::new(Audit::default()))
}

#[derive(Debug)]
pub struct AffineResource {
    id: u64,
    open: bool,
    audit: SharedAudit,
}

impl AffineResource {
    pub fn new(id: u64, audit: SharedAudit) -> Self {
        audit
            .borrow_mut()
            .events
            .push(format!("resource:{id}:open"));
        Self {
            id,
            open: true,
            audit,
        }
    }

    pub fn borrow(&self) -> BorrowedResource<'_> {
        BorrowedResource { resource: self }
    }

    pub fn close(mut self) {
        if self.open {
            let mut audit = self.audit.borrow_mut();
            audit.closes += 1;
            audit.events.push(format!("resource:{}:close", self.id));
            self.open = false;
        }
    }
}

impl Drop for AffineResource {
    fn drop(&mut self) {
        if self.open {
            let mut audit = self.audit.borrow_mut();
            audit.drops += 1;
            audit.events.push(format!("resource:{}:drop", self.id));
            self.open = false;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BorrowedResource<'a> {
    resource: &'a AffineResource,
}

impl BorrowedResource<'_> {
    pub fn id(self) -> u64 {
        self.resource.id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskState {
    Pending,
    Completed,
    Cancelled,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TaskError {
    Cancelled,
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "task cancelled")
    }
}

impl std::error::Error for TaskError {}

#[derive(Debug)]
pub struct TaskScope {
    audit: SharedAudit,
    children: RefCell<Vec<Weak<RefCell<TaskState>>>>,
    cancelled: Cell<bool>,
}

impl TaskScope {
    pub fn new(audit: SharedAudit) -> Self {
        audit.borrow_mut().events.push("scope:open".to_owned());
        Self {
            audit,
            children: RefCell::new(Vec::new()),
            cancelled: Cell::new(false),
        }
    }

    pub fn spawn<T>(&self, name: impl Into<String>, value: T) -> ScopedTask<'_, T> {
        let state = Rc::new(RefCell::new(TaskState::Pending));
        let name = name.into();
        self.children.borrow_mut().push(Rc::downgrade(&state));
        self.audit
            .borrow_mut()
            .events
            .push(format!("task:{name}:spawn"));
        ScopedTask {
            name,
            value: Some(value),
            state,
            audit: Rc::clone(&self.audit),
            _scope: PhantomData,
        }
    }

    pub fn cancel_all(&self) {
        if self.cancelled.replace(true) {
            return;
        }
        for weak in self.children.borrow().iter() {
            if let Some(state) = weak.upgrade() {
                let mut state = state.borrow_mut();
                if *state == TaskState::Pending {
                    *state = TaskState::Cancelled;
                    self.audit.borrow_mut().cancellations += 1;
                }
            }
        }
        self.audit
            .borrow_mut()
            .events
            .push("scope:cancel".to_owned());
    }
}

impl Drop for TaskScope {
    fn drop(&mut self) {
        self.cancel_all();
        self.audit.borrow_mut().events.push("scope:drop".to_owned());
    }
}

#[derive(Debug)]
pub struct ScopedTask<'scope, T> {
    name: String,
    value: Option<T>,
    state: Rc<RefCell<TaskState>>,
    audit: SharedAudit,
    _scope: PhantomData<&'scope TaskScope>,
}

impl<T> ScopedTask<'_, T> {
    pub fn finish(mut self) -> Result<T, TaskError> {
        if *self.state.borrow() == TaskState::Cancelled {
            return Err(TaskError::Cancelled);
        }
        *self.state.borrow_mut() = TaskState::Completed;
        let mut audit = self.audit.borrow_mut();
        audit.completions += 1;
        audit.events.push(format!("task:{}:complete", self.name));
        drop(audit);
        Ok(self.value.take().expect("pending task contains a value"))
    }
}

impl<T> Drop for ScopedTask<'_, T> {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        if *state == TaskState::Pending {
            *state = TaskState::Cancelled;
            let mut audit = self.audit.borrow_mut();
            audit.cancellations += 1;
            audit.events.push(format!("task:{}:drop-cancel", self.name));
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FutureError {
    Pending,
    Cancelled,
    AlreadyCompleted,
}

impl std::fmt::Display for FutureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "future {self:?}")
    }
}

impl std::error::Error for FutureError {}

#[derive(Debug)]
enum FutureState<T> {
    Pending,
    Ready(T),
    Cancelled,
    Consumed,
}

#[derive(Debug)]
pub struct FutureProducer<T> {
    state: Rc<RefCell<FutureState<T>>>,
}

#[derive(Debug)]
pub struct FutureReader<T> {
    state: Rc<RefCell<FutureState<T>>>,
    audit: SharedAudit,
}

pub fn one_shot<T>(audit: SharedAudit) -> (FutureProducer<T>, FutureReader<T>) {
    let state = Rc::new(RefCell::new(FutureState::Pending));
    (
        FutureProducer {
            state: Rc::clone(&state),
        },
        FutureReader { state, audit },
    )
}

impl<T> FutureProducer<T> {
    pub fn complete(self, value: T) -> Result<(), FutureError> {
        let mut state = self.state.borrow_mut();
        match &*state {
            FutureState::Pending => {
                *state = FutureState::Ready(value);
                Ok(())
            }
            FutureState::Cancelled => Err(FutureError::Cancelled),
            _ => Err(FutureError::AlreadyCompleted),
        }
    }
}

impl<T> FutureReader<T> {
    pub fn read(self) -> Result<T, FutureError> {
        let mut state = self.state.borrow_mut();
        match std::mem::replace(&mut *state, FutureState::Consumed) {
            FutureState::Ready(value) => Ok(value),
            FutureState::Pending => {
                *state = FutureState::Pending;
                Err(FutureError::Pending)
            }
            FutureState::Cancelled => Err(FutureError::Cancelled),
            FutureState::Consumed => Err(FutureError::AlreadyCompleted),
        }
    }
}

impl<T> Drop for FutureReader<T> {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        if matches!(*state, FutureState::Pending | FutureState::Ready(_)) {
            *state = FutureState::Cancelled;
            let mut audit = self.audit.borrow_mut();
            audit.cancellations += 1;
            audit.events.push("future:reader-close".to_owned());
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamError {
    Backpressure,
    Closed,
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stream {self:?}")
    }
}

impl std::error::Error for StreamError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pull<T> {
    Item(T),
    Pending,
    End,
}

#[derive(Debug)]
pub struct BoundedStream<T> {
    queue: VecDeque<T>,
    capacity: usize,
    closed: bool,
    audit: SharedAudit,
}

impl<T> BoundedStream<T> {
    pub fn new(capacity: usize, audit: SharedAudit) -> Self {
        assert!(capacity > 0, "stream capacity must be positive");
        Self {
            queue: VecDeque::with_capacity(capacity),
            capacity,
            closed: false,
            audit,
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), StreamError> {
        if self.closed {
            return Err(StreamError::Closed);
        }
        if self.queue.len() == self.capacity {
            self.audit.borrow_mut().backpressure += 1;
            return Err(StreamError::Backpressure);
        }
        self.queue.push_back(value);
        let len = self.queue.len();
        let mut audit = self.audit.borrow_mut();
        audit.max_in_flight = audit.max_in_flight.max(len);
        Ok(())
    }

    pub fn pull(&mut self) -> Pull<T> {
        match self.queue.pop_front() {
            Some(value) => Pull::Item(value),
            None if self.closed => Pull::End,
            None => Pull::Pending,
        }
    }

    pub fn close(&mut self) {
        self.closed = true;
    }
}

impl<T> Drop for BoundedStream<T> {
    fn drop(&mut self) {
        if !self.closed {
            self.closed = true;
            let mut audit = self.audit.borrow_mut();
            audit.cancellations += 1;
            audit.events.push("stream:drop-cancel".to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_close_does_not_double_drop() {
        let audit = new_audit();
        AffineResource::new(1, Rc::clone(&audit)).close();
        assert_eq!((audit.borrow().closes, audit.borrow().drops), (1, 0));
    }

    #[test]
    fn error_path_drops_resource_once() {
        fn fail(audit: SharedAudit) -> Result<(), &'static str> {
            let _resource = AffineResource::new(2, audit);
            Err("expected")
        }
        let audit = new_audit();
        assert_eq!(fail(Rc::clone(&audit)), Err("expected"));
        assert_eq!((audit.borrow().closes, audit.borrow().drops), (0, 1));
    }

    #[test]
    fn borrow_observes_without_consuming() {
        let audit = new_audit();
        let resource = AffineResource::new(3, Rc::clone(&audit));
        assert_eq!(resource.borrow().id(), 3);
        resource.close();
        assert_eq!(audit.borrow().closes, 1);
    }

    #[test]
    fn task_finishes_once_by_consuming_handle() {
        let audit = new_audit();
        let scope = TaskScope::new(Rc::clone(&audit));
        let task = scope.spawn("answer", 42);
        assert_eq!(task.finish(), Ok(42));
        assert_eq!(audit.borrow().completions, 1);
    }

    #[test]
    fn scope_cancellation_reaches_pending_child() {
        let audit = new_audit();
        let scope = TaskScope::new(Rc::clone(&audit));
        let task = scope.spawn("pending", 42);
        scope.cancel_all();
        assert_eq!(task.finish(), Err(TaskError::Cancelled));
        assert_eq!(audit.borrow().cancellations, 1);
    }

    #[test]
    fn dropping_pending_task_cancels_it() {
        let audit = new_audit();
        let scope = TaskScope::new(Rc::clone(&audit));
        drop(scope.spawn("pending", 42));
        assert_eq!(audit.borrow().cancellations, 1);
    }

    #[test]
    fn one_shot_future_yields_one_value() {
        let audit = new_audit();
        let (producer, reader) = one_shot(Rc::clone(&audit));
        producer.complete(7).unwrap();
        assert_eq!(reader.read(), Ok(7));
        assert_eq!(audit.borrow().cancellations, 0);
    }

    #[test]
    fn dropping_future_reader_cancels_producer() {
        let audit = new_audit();
        let (producer, reader) = one_shot::<u8>(Rc::clone(&audit));
        drop(reader);
        assert_eq!(producer.complete(7), Err(FutureError::Cancelled));
        assert_eq!(audit.borrow().cancellations, 1);
    }

    #[test]
    fn bounded_stream_applies_backpressure_and_ends() {
        let audit = new_audit();
        let mut stream = BoundedStream::new(2, Rc::clone(&audit));
        stream.push(1).unwrap();
        stream.push(2).unwrap();
        assert_eq!(stream.push(3), Err(StreamError::Backpressure));
        assert_eq!(stream.pull(), Pull::Item(1));
        stream.push(3).unwrap();
        stream.close();
        assert_eq!(stream.pull(), Pull::Item(2));
        assert_eq!(stream.pull(), Pull::Item(3));
        assert_eq!(stream.pull(), Pull::End);
        assert_eq!(
            (audit.borrow().max_in_flight, audit.borrow().backpressure),
            (2, 1)
        );
    }

    #[test]
    fn dropping_open_stream_cancels_it() {
        let audit = new_audit();
        let mut stream = BoundedStream::new(1, Rc::clone(&audit));
        stream.push(1).unwrap();
        drop(stream);
        assert_eq!(audit.borrow().cancellations, 1);
    }
}
