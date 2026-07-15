use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
const MAX_OPEN_EVENTS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeBackend {
    NativeCranelift,
    Pulley,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendAvailability {
    pub native_compiled: bool,
    pub executable_memory: bool,
    pub pulley_compiled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MobileLifecycleError {
    RuntimeUnavailable,
    InvalidDescriptor,
    InvalidTransition,
    OpenQueueFull,
    Terminal,
}

impl std::fmt::Display for MobileLifecycleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Android lifecycle rejected: {self:?}")
    }
}

impl std::error::Error for MobileLifecycleError {}

/// Selects native compilation only when executable memory is proven, otherwise
/// falls back to the portable Pulley interpreter.
///
/// # Errors
///
/// Returns `RuntimeUnavailable` when neither backend is compiled and usable.
pub fn select_runtime_backend(
    availability: BackendAvailability,
) -> Result<RuntimeBackend, MobileLifecycleError> {
    if availability.native_compiled && availability.executable_memory {
        Ok(RuntimeBackend::NativeCranelift)
    } else if availability.pulley_compiled {
        Ok(RuntimeBackend::Pulley)
    } else {
        Err(MobileLifecycleError::RuntimeUnavailable)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineConfigPlan {
    pub component_model: bool,
    pub compiler: &'static str,
    pub target: &'static str,
}

/// Returns the pinned Wasmtime configuration that the Android JNI layer must
/// apply once the licensed Android build environment is available.
#[must_use]
pub const fn engine_config_plan(backend: RuntimeBackend) -> EngineConfigPlan {
    EngineConfigPlan {
        component_model: true,
        compiler: "cranelift",
        target: match backend {
            RuntimeBackend::NativeCranelift => "android-native",
            RuntimeBackend::Pulley => "pulley64",
        },
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleDescriptor {
    pub app_identity: String,
    pub revision_digest: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AndroidGuestState {
    Created,
    Foreground,
    BackgroundSuspended,
    Stopping,
    Terminal(TerminalReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalReason {
    Exited,
    Crashed,
    TimedOut,
    Cancelled,
    ProcessDied,
}

pub struct AndroidLifecycle {
    descriptor: LifecycleDescriptor,
    state: AndroidGuestState,
    queued_revision_opens: VecDeque<String>,
}

impl AndroidLifecycle {
    /// Creates lifecycle state from immutable identities only.
    ///
    /// # Errors
    ///
    /// Rejects malformed app/revision digests.
    pub fn create(descriptor: LifecycleDescriptor) -> Result<Self, MobileLifecycleError> {
        if !is_digest(&descriptor.app_identity) || !is_digest(&descriptor.revision_digest) {
            return Err(MobileLifecycleError::InvalidDescriptor);
        }
        Ok(Self {
            descriptor,
            state: AndroidGuestState::Created,
            queued_revision_opens: VecDeque::new(),
        })
    }

    #[must_use]
    pub const fn state(&self) -> AndroidGuestState {
        self.state
    }

    #[must_use]
    pub fn descriptor(&self) -> &LifecycleDescriptor {
        &self.descriptor
    }

    /// Foregrounds a newly created or suspended guest.
    ///
    /// # Errors
    ///
    /// Rejects stopping or terminal states.
    pub fn foreground(&mut self) -> Result<(), MobileLifecycleError> {
        match self.state {
            AndroidGuestState::Created | AndroidGuestState::BackgroundSuspended => {
                self.state = AndroidGuestState::Foreground;
                Ok(())
            }
            AndroidGuestState::Foreground => Ok(()),
            AndroidGuestState::Stopping | AndroidGuestState::Terminal(_) => {
                Err(MobileLifecycleError::InvalidTransition)
            }
        }
    }

    /// Suspends guest work when the Activity becomes invisible.
    ///
    /// # Errors
    ///
    /// Requires a foreground guest.
    pub fn background(&mut self) -> Result<(), MobileLifecycleError> {
        if self.state != AndroidGuestState::Foreground {
            return Err(MobileLifecycleError::InvalidTransition);
        }
        self.state = AndroidGuestState::BackgroundSuspended;
        Ok(())
    }

    /// Queues one immutable duplicate-open revision.
    ///
    /// # Errors
    ///
    /// Rejects malformed revisions, terminal state and the 257th event.
    pub fn queue_open(&mut self, revision_digest: String) -> Result<(), MobileLifecycleError> {
        if matches!(self.state, AndroidGuestState::Terminal(_)) {
            return Err(MobileLifecycleError::Terminal);
        }
        if !is_digest(&revision_digest) {
            return Err(MobileLifecycleError::InvalidDescriptor);
        }
        if self.queued_revision_opens.len() >= MAX_OPEN_EVENTS {
            return Err(MobileLifecycleError::OpenQueueFull);
        }
        self.queued_revision_opens.push_back(revision_digest);
        Ok(())
    }

    pub fn take_open(&mut self) -> Option<String> {
        self.queued_revision_opens.pop_front()
    }

    /// Records exactly one terminal outcome.
    ///
    /// # Errors
    ///
    /// Rejects all later terminal races.
    pub fn finish(&mut self, reason: TerminalReason) -> Result<(), MobileLifecycleError> {
        if matches!(self.state, AndroidGuestState::Terminal(_)) {
            return Err(MobileLifecycleError::Terminal);
        }
        self.state = AndroidGuestState::Stopping;
        self.queued_revision_opens.clear();
        self.state = AndroidGuestState::Terminal(reason);
        Ok(())
    }

    /// Restores after process death as a non-running descriptor requiring
    /// package re-verification.
    #[must_use]
    pub fn restore_after_process_death(descriptor: LifecycleDescriptor) -> Option<Self> {
        Self::create(descriptor).ok()
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
