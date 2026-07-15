use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::{PermissionSession, is_link_like};

pub const MAX_QUEUED_OPEN_EVENTS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuestState {
    Starting,
    Running,
    Background,
    Closing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenRequest {
    pub package_path: PathBuf,
}

impl OpenRequest {
    /// Constructs a bounded `.sapp` open event without interpreting the path
    /// as a command line.
    ///
    /// # Errors
    ///
    /// Rejects non-`.sapp` or excessively long paths.
    pub fn new(package_path: PathBuf) -> Result<Self, LifecycleError> {
        if package_path.as_os_str().len() > 32_768
            || !package_path
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .is_some_and(|extension| extension.eq_ignore_ascii_case("sapp"))
        {
            return Err(LifecycleError::InvalidOpenRequest);
        }
        Ok(Self { package_path })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    pub program: OsString,
    pub arguments: Vec<OsString>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LaunchOutcome {
    Started,
    AlreadyRunning,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalOutcome {
    Exited(Option<i32>),
    Crashed(Option<i32>),
    TimedOut,
    Cancelled,
}

#[derive(Debug)]
pub enum LifecycleError {
    Io(std::io::Error),
    InvalidIdentity,
    InvalidOpenRequest,
    NotRunning,
    EventQueueFull,
    InvalidTransition,
    UnsafeCleanup,
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "guest lifecycle I/O failed: {error}"),
            Self::InvalidIdentity => formatter.write_str("guest identity is invalid"),
            Self::InvalidOpenRequest => formatter.write_str("open request is invalid"),
            Self::NotRunning => formatter.write_str("guest is not running"),
            Self::EventQueueFull => formatter.write_str("guest open event queue is full"),
            Self::InvalidTransition => formatter.write_str("guest lifecycle transition is invalid"),
            Self::UnsafeCleanup => formatter.write_str("guest cleanup path is unsafe"),
        }
    }
}

impl std::error::Error for LifecycleError {}

impl From<std::io::Error> for LifecycleError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

struct ActiveGuest {
    child: Child,
    state: GuestState,
    opens: VecDeque<OpenRequest>,
    permission_session: PermissionSession,
    cleanup_files: Vec<PathBuf>,
}

pub struct ProcessSupervisor {
    cleanup_root: PathBuf,
    active: BTreeMap<String, ActiveGuest>,
}

impl ProcessSupervisor {
    /// Creates a supervisor with a Host-owned cleanup root.
    ///
    /// # Errors
    ///
    /// Rejects link/reparse roots and I/O failures.
    pub fn open(cleanup_root: &Path) -> Result<Self, LifecycleError> {
        fs::create_dir_all(cleanup_root)?;
        reject_directory(cleanup_root)?;
        Ok(Self {
            cleanup_root: cleanup_root.canonicalize()?,
            active: BTreeMap::new(),
        })
    }

    /// Starts one child for an immutable app identity. Program and arguments
    /// are passed directly to the OS process API, never through a shell string.
    ///
    /// # Errors
    ///
    /// Rejects invalid identities and process spawn failures.
    pub fn launch(
        &mut self,
        app_identity: &str,
        command: &CommandSpec,
    ) -> Result<LaunchOutcome, LifecycleError> {
        validate_identity(app_identity)?;
        if self.active.contains_key(app_identity) {
            return Ok(LaunchOutcome::AlreadyRunning);
        }
        let child = Command::new(&command.program)
            .args(&command.arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        self.active.insert(
            app_identity.to_owned(),
            ActiveGuest {
                child,
                state: GuestState::Running,
                opens: VecDeque::new(),
                permission_session: PermissionSession::default(),
                cleanup_files: Vec::new(),
            },
        );
        Ok(LaunchOutcome::Started)
    }

    /// Queues a duplicate-open event for the existing supervised guest.
    ///
    /// # Errors
    ///
    /// Rejects missing guests and queues beyond the fixed ceiling.
    pub fn queue_open(
        &mut self,
        app_identity: &str,
        request: OpenRequest,
    ) -> Result<(), LifecycleError> {
        let guest = self
            .active
            .get_mut(app_identity)
            .ok_or(LifecycleError::NotRunning)?;
        if guest.opens.len() >= MAX_QUEUED_OPEN_EVENTS {
            return Err(LifecycleError::EventQueueFull);
        }
        guest.opens.push_back(request);
        Ok(())
    }

    /// Takes the oldest queued open event.
    ///
    /// # Errors
    ///
    /// Returns `NotRunning` when the immutable app identity has no guest.
    pub fn take_open(&mut self, app_identity: &str) -> Result<Option<OpenRequest>, LifecycleError> {
        Ok(self
            .active
            .get_mut(app_identity)
            .ok_or(LifecycleError::NotRunning)?
            .opens
            .pop_front())
    }

    /// Moves a running guest between foreground and background state.
    ///
    /// # Errors
    ///
    /// Rejects missing guests and invalid transitions.
    pub fn set_background(
        &mut self,
        app_identity: &str,
        background: bool,
    ) -> Result<(), LifecycleError> {
        let guest = self
            .active
            .get_mut(app_identity)
            .ok_or(LifecycleError::NotRunning)?;
        match (guest.state, background) {
            (GuestState::Running, true) => guest.state = GuestState::Background,
            (GuestState::Background, false) => guest.state = GuestState::Running,
            (GuestState::Running, false) | (GuestState::Background, true) => {}
            _ => return Err(LifecycleError::InvalidTransition),
        }
        Ok(())
    }

    /// Registers an exact existing regular file beneath the cleanup root.
    ///
    /// # Errors
    ///
    /// Rejects links, directories and paths escaping the cleanup root.
    pub fn register_cleanup_file(
        &mut self,
        app_identity: &str,
        path: &Path,
    ) -> Result<(), LifecycleError> {
        let canonical = path.canonicalize()?;
        if !canonical.starts_with(&self.cleanup_root) {
            return Err(LifecycleError::UnsafeCleanup);
        }
        let metadata = fs::symlink_metadata(&canonical)?;
        if is_link_like(&metadata) || !metadata.is_file() {
            return Err(LifecycleError::UnsafeCleanup);
        }
        self.active
            .get_mut(app_identity)
            .ok_or(LifecycleError::NotRunning)?
            .cleanup_files
            .push(canonical);
        Ok(())
    }

    /// Polls without blocking and performs terminal cleanup on exit.
    ///
    /// # Errors
    ///
    /// Rejects missing guests and process/cleanup I/O failures.
    pub fn poll(&mut self, app_identity: &str) -> Result<Option<TerminalOutcome>, LifecycleError> {
        let status = self
            .active
            .get_mut(app_identity)
            .ok_or(LifecycleError::NotRunning)?
            .child
            .try_wait()?;
        status.map_or(Ok(None), |status| {
            self.finish(app_identity, terminal_from_status(status))
                .map(Some)
        })
    }

    /// Waits until exit or the deadline, then kills the process tree and
    /// classifies the outcome as timeout.
    ///
    /// # Errors
    ///
    /// Rejects missing guests and process/cleanup I/O failures.
    pub fn wait_with_timeout(
        &mut self,
        app_identity: &str,
        timeout: Duration,
    ) -> Result<TerminalOutcome, LifecycleError> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(outcome) = self.poll(app_identity)? {
                return Ok(outcome);
            }
            if Instant::now() >= deadline {
                let guest = self
                    .active
                    .get_mut(app_identity)
                    .ok_or(LifecycleError::NotRunning)?;
                guest.state = GuestState::Closing;
                kill_process_tree(&mut guest.child);
                let _ = guest.child.wait();
                return self.finish(app_identity, TerminalOutcome::TimedOut);
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// Cancels and kills the supervised process tree.
    ///
    /// # Errors
    ///
    /// Rejects missing guests and cleanup I/O failures.
    pub fn cancel(&mut self, app_identity: &str) -> Result<TerminalOutcome, LifecycleError> {
        let guest = self
            .active
            .get_mut(app_identity)
            .ok_or(LifecycleError::NotRunning)?;
        guest.state = GuestState::Closing;
        kill_process_tree(&mut guest.child);
        let _ = guest.child.wait();
        self.finish(app_identity, TerminalOutcome::Cancelled)
    }

    #[must_use]
    pub fn is_running(&self, app_identity: &str) -> bool {
        self.active.contains_key(app_identity)
    }

    fn finish(
        &mut self,
        app_identity: &str,
        outcome: TerminalOutcome,
    ) -> Result<TerminalOutcome, LifecycleError> {
        let mut guest = self
            .active
            .remove(app_identity)
            .ok_or(LifecycleError::NotRunning)?;
        guest.permission_session.clear();
        for path in guest.cleanup_files {
            let metadata = fs::symlink_metadata(&path)?;
            if is_link_like(&metadata)
                || !metadata.is_file()
                || !path.starts_with(&self.cleanup_root)
            {
                return Err(LifecycleError::UnsafeCleanup);
            }
            fs::remove_file(path)?;
        }
        Ok(outcome)
    }
}

fn terminal_from_status(status: ExitStatus) -> TerminalOutcome {
    if status.success() {
        TerminalOutcome::Exited(status.code())
    } else {
        TerminalOutcome::Crashed(status.code())
    }
}

fn validate_identity(identity: &str) -> Result<(), LifecycleError> {
    if identity.len() == 64
        && identity
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(LifecycleError::InvalidIdentity)
    }
}

#[cfg(windows)]
fn kill_process_tree(child: &mut Child) {
    let _ = Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
}

#[cfg(not(windows))]
fn kill_process_tree(child: &mut Child) {
    let _ = child.kill();
}

fn reject_directory(path: &Path) -> Result<(), LifecycleError> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link_like(&metadata) || !metadata.is_dir() {
        Err(LifecycleError::UnsafeCleanup)
    } else {
        Ok(())
    }
}
