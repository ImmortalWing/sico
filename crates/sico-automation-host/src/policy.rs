//! The single authority composition point (RFC-0040 §3): a run's
//! `AutomationPolicy` is the complete, explicit grant set. Nothing outside
//! it exists for the run; grants never compose by inference.

/// Logical-window rate limits (a real backend maps ticks to instants).
#[derive(Clone, Debug)]
pub struct AutomationPolicy {
    /// `capture` grant. Without it, capture is a typed `Permission`.
    pub capture_granted: bool,
    /// `input.pointer` grant.
    pub pointer_granted: bool,
    /// `input.keyboard` grant — deliberately separate from pointer.
    pub keyboard_granted: bool,
    /// bgra8 frame byte ceiling.
    pub max_frame_bytes: u64,
    /// Frames per rate window.
    pub max_frames_per_window: usize,
    /// Total frames per surface per run.
    pub max_total_frames: u64,
    /// Preview tokens per rate window (threat T4: review-fatigue).
    pub max_tokens_per_window: usize,
    /// Committed actions per surface per run.
    pub max_actions_per_surface: u64,
    /// Rate window length in logical ticks.
    pub rate_window_ticks: u64,
    /// Token time-to-live in logical ticks.
    pub token_ttl_ticks: u64,
}

impl Default for AutomationPolicy {
    fn default() -> Self {
        Self {
            capture_granted: true,
            pointer_granted: true,
            keyboard_granted: false,
            max_frame_bytes: 64 * 1024 * 1024,
            max_frames_per_window: 60,
            max_total_frames: 1_000_000,
            max_tokens_per_window: 32,
            max_actions_per_surface: 4_096,
            rate_window_ticks: 64,
            token_ttl_ticks: 16,
        }
    }
}
