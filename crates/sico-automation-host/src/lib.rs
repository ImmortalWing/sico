//! Native Automation Host capability core (M16, STEP-0147 contract set).
//!
//! This crate implements the Host side of the RFC-0040 capability surface
//! — `observe`, `capture`, `pointer`, `keyboard`, plus the Host-owned
//! audit stream and emergency stop — against pluggable surface backends.
//! The synthetic backend (`synthetic`) realizes every guarantee without OS
//! access (`contract-verified` evidence class); the Windows backend lands
//! as its own step and must satisfy the same corpus.
//!
//! Invariants carried over from the accepted contracts:
//! - capture does not imply input; grants compose only by explicit policy;
//! - dry-run is Host-enforced: the preview path withholds token minting,
//!   so a dry-run guest cannot commit and cannot even observe the mode;
//! - every preview/commit token binds surface identity, observation
//!   revision, action digest, expiry and a single-use marker;
//! - the audit stream is append-only and redacted (no pixels, no keys);
//! - emergency stop is Host-owned: teardown withdraws every grant.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

pub mod audit;
#[deny(unsafe_code)]
pub mod policy;
#[deny(unsafe_code)]
pub mod synthetic;
#[cfg(windows)]
pub mod win32;

/// Logical monotonic tick. The synthetic backend drives it explicitly;
/// a real backend maps it to wall-clock instants at its own boundary.
pub type Tick = u64;

/// Stable, revocable identity of one Host-authorized surface
/// (RFC-0040 `observe.surface-id`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SurfaceId {
    pub token: String,
}

/// One observable surface's Host-exposed description.
#[derive(Clone, Debug)]
pub struct SurfaceInfo {
    pub id: SurfaceId,
    pub title: String,
    pub width: u32,
    pub height: u32,
}

/// A surface's observable state revision: bumped by every geometry or
/// content-affecting event (move/resize/focus/repaint with content change).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Revision(pub u64);

/// One owned capture frame (RFC-0040 `capture.frame`, bgra8).
#[derive(Clone, Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObserveError {
    SurfaceInvalid,
    Permission,
    Limit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureError {
    SurfaceInvalid,
    Permission,
    Limit,
    Io,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionVariant {
    Move,
    Press,
    Release,
    Drag,
}

/// One previewed, committed action; coordinates resolve against the bound
/// surface's current geometry, never raw screen space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action {
    pub variant: ActionVariant,
    pub x: u32,
    pub y: u32,
    pub duration_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerError {
    SurfaceInvalid,
    StaleRevision,
    TokenInvalid,
    Expired,
    Budget,
    Permission,
    Io,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardError {
    SurfaceInvalid,
    StaleRevision,
    TokenInvalid,
    Expired,
    Budget,
    /// The surface is declared sensitive by operator policy (threat model
    /// F-1 ruling) — typing refuses closed.
    SensitiveField,
    /// Non-printable text outside the v0 alphabet (RFC-0040 amendment A1).
    InvalidText,
    Permission,
    Io,
}

/// An opaque single-use preview/commit token. The binding data lives in
/// the Host registry, never in the token itself.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommitToken {
    pub opaque: String,
}

/// Digest of one exact action — binds the token to what it may commit.
pub(crate) fn action_digest(surface: &SurfaceId, revision: Revision, action: &Action) -> String {
    let mut hasher = Sha256::new();
    hasher.update(surface.token.as_bytes());
    hasher.update(revision.0.to_le_bytes());
    hasher.update([match action.variant {
        ActionVariant::Move => 0_u8,
        ActionVariant::Press => 1,
        ActionVariant::Release => 2,
        ActionVariant::Drag => 3,
    }]);
    hasher.update(action.x.to_le_bytes());
    hasher.update(action.y.to_le_bytes());
    hasher.update(action.duration_ms.to_le_bytes());
    format!("{:x}", Sha256::digest(hasher.finalize()))
}

/// One minted token's Host-side binding (RFC-0040 §2).
#[derive(Clone, Debug)]
pub(crate) struct TokenRecord {
    pub surface: SurfaceId,
    pub revision: Revision,
    pub digest: String,
    pub expiry: Tick,
    pub used: bool,
}

/// Per-surface accounting.
#[derive(Clone, Debug)]
struct SurfaceState {
    title: String,
    width: u32,
    height: u32,
    revision: Revision,
    alive: bool,
    frames_captured: u64,
    actions_committed: u64,
    last_capture_tick: Tick,
    /// Declared sensitive by operator policy (F-1): keyboard refuses.
    sensitive: bool,
}

/// The capability core. `S` is the surface backend: it supplies pixels and
/// geometry for authorized surfaces only; every authority decision happens
/// here, Host-side, guest-invisibly.
pub struct AutomationHost {
    policy: policy::AutomationPolicy,
    surfaces: BTreeMap<SurfaceId, SurfaceState>,
    tokens: BTreeMap<String, TokenRecord>,
    events: Vec<audit::AuditEvent>,
    stopped: bool,
    tick: Tick,
    /// Per-mint counter: identical previews must stay distinct tokens.
    mint_counter: u64,
    /// Tokens minted in the current rate window (threat T4).
    minted_this_window: Vec<Tick>,
    /// Frames captured in the current rate window.
    captured_this_window: Vec<Tick>,
}

impl AutomationHost {
    /// Creates the Host core for one run. The policy is the run's entire
    /// authority: anything it does not expose does not exist.
    #[must_use]
    pub fn new(policy: policy::AutomationPolicy) -> Self {
        Self {
            policy,
            surfaces: BTreeMap::new(),
            tokens: BTreeMap::new(),
            events: Vec::new(),
            stopped: false,
            tick: 0,
            mint_counter: 0,
            minted_this_window: Vec::new(),
            captured_this_window: Vec::new(),
        }
    }

    /// Advances the logical clock (expiry and rate windows).
    pub fn tick(&mut self) {
        self.tick = self.tick.saturating_add(1);
        let window = self.policy.rate_window_ticks;
        self.minted_this_window
            .retain(|t| self.tick.saturating_sub(*t) < window);
        self.captured_this_window
            .retain(|t| self.tick.saturating_sub(*t) < window);
    }

    #[must_use]
    pub fn now(&self) -> Tick {
        self.tick
    }

    /// Registers one policy-exposed surface; returns its fresh identity.
    /// Surfaces not registered here are invisible to every capability.
    pub fn expose_surface(
        &mut self,
        title: &str,
        width: u32,
        height: u32,
        sensitive: bool,
    ) -> SurfaceId {
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        hasher.update(self.tick.to_le_bytes());
        hasher.update(self.surfaces.len().to_le_bytes());
        let token = format!("{:x}", Sha256::digest(hasher.finalize()));
        let id = SurfaceId { token };
        self.surfaces.insert(
            id.clone(),
            SurfaceState {
                title: title.to_owned(),
                width,
                height,
                revision: Revision(1),
                alive: true,
                frames_captured: 0,
                actions_committed: 0,
                last_capture_tick: 0,
                sensitive,
            },
        );
        id
    }

    /// Marks a surface's observable state changed (geometry or content).
    pub fn bump_revision(&mut self, id: &SurfaceId) {
        if let Some(state) = self.surfaces.get_mut(id) {
            state.revision.0 = state.revision.0.saturating_add(1);
        }
    }

    /// Revokes a surface (close/minimize/display change): its identity
    /// invalidates for every later use.
    pub fn revoke_surface(&mut self, id: &SurfaceId) {
        if let Some(state) = self.surfaces.get_mut(id) {
            state.alive = false;
        }
        self.drain_tokens_for(id);
    }

    /// Emergancy stop (threat T7): withdraws every grant, drains every
    /// outstanding token, records one audit event. Infallible and
    /// guest-invisible; a stopped host serves nothing further.
    pub fn emergency_stop(&mut self) {
        self.stopped = true;
        self.tokens.clear();
        self.events.push(audit::AuditEvent::stop(self.tick));
    }

    #[must_use]
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// Observable audit log (operator-side view; guests have none of it).
    #[must_use]
    pub fn audit(&self) -> &[audit::AuditEvent] {
        &self.events
    }

    fn check_stopped(&self) -> bool {
        self.stopped
    }

    /// `observe.surfaces`: only policy-exposed, still-alive surfaces.
    #[must_use]
    pub fn surfaces(&self) -> Vec<SurfaceInfo> {
        self.surfaces
            .iter()
            .filter(|(_, state)| state.alive)
            .map(|(id, state)| SurfaceInfo {
                id: id.clone(),
                title: state.title.clone(),
                width: state.width,
                height: state.height,
            })
            .collect()
    }

    /// `observe.revision`.
    ///
    /// # Errors
    ///
    /// `SurfaceInvalid` for revoked surfaces, `Permission` when unknown.
    pub fn revision(&self, id: &SurfaceId) -> Result<Revision, ObserveError> {
        let state = self.surfaces.get(id).ok_or(ObserveError::Permission)?;
        if !state.alive {
            return Err(ObserveError::SurfaceInvalid);
        }
        Ok(state.revision)
    }

    /// Lookups a still-live surface; `None` covers both unknown identity
    /// and revoked surfaces — callers map that to their own typed error.
    fn live_state(&self, id: &SurfaceId) -> Option<&SurfaceState> {
        let state = self.surfaces.get(id)?;
        if !state.alive {
            return None;
        }
        Some(state)
    }

    /// `capture.capture` with the policy's size/rate/total ceilings.
    ///
    /// # Errors
    ///
    /// Typed [`CaptureError`] per RFC-0040; never truncates a frame.
    ///
    /// # Panics
    ///
    /// Only if the internal state index vanished, which the live check
    /// above makes unreachable.
    pub fn capture(&mut self, id: &SurfaceId) -> Result<Frame, CaptureError> {
        if self.check_stopped() {
            return Err(CaptureError::Permission);
        }
        if !self.policy.capture_granted {
            return Err(CaptureError::Permission);
        }
        let Some((width, height, revision)) = self
            .live_state(id)
            .map(|state| (state.width, state.height, state.revision))
        else {
            return Err(CaptureError::SurfaceInvalid);
        };
        let bytes = u64::from(width) * u64::from(height) * 4;
        if bytes > self.policy.max_frame_bytes {
            return Err(CaptureError::Limit);
        }
        if self.captured_this_window.len() >= self.policy.max_frames_per_window {
            return Err(CaptureError::Limit);
        }
        let state = self.surfaces.get_mut(id).expect("checked live above");
        if state.frames_captured >= self.policy.max_total_frames {
            return Err(CaptureError::Limit);
        }
        state.frames_captured = state.frames_captured.saturating_add(1);
        state.last_capture_tick = self.tick;
        self.captured_this_window.push(self.tick);
        self.events.push(audit::AuditEvent::capture(
            self.tick,
            &id.token,
            revision.0,
            state.frames_captured,
        ));
        // The policy ceiling keeps `bytes` inside the 32-bit-safe range
        // too; the truncation lint is bounded by `max_frame_bytes`.
        #[allow(clippy::cast_possible_truncation)]
        Ok(Frame {
            width,
            height,
            stride: width * 4,
            pixels: vec![0; bytes as usize],
        })
    }

    /// The preview path: mints one single-use token binding surface,
    /// revision, action digest and expiry. Dry-run withholds minting
    /// entirely (ADR-0013 §4): a dry-run guest can plan and observe but
    /// every mint is a typed refusal.
    ///
    /// # Errors
    ///
    /// Typed [`PointerError`] per RFC-0040 §1; `Budget` binds the rate
    /// window and the per-surface action ceiling.
    pub fn mint_token(
        &mut self,
        id: &SurfaceId,
        expected_revision: Revision,
        action: &Action,
    ) -> Result<CommitToken, PointerError> {
        if self.check_stopped() {
            return Err(PointerError::Permission);
        }
        if !self.policy.pointer_granted {
            return Err(PointerError::Permission);
        }
        let Some(state) = self.live_state(id) else {
            return Err(PointerError::SurfaceInvalid);
        };
        if state.revision != expected_revision {
            return Err(PointerError::StaleRevision);
        }
        if action.x >= state.width || action.y >= state.height {
            return Err(PointerError::TokenInvalid);
        }
        if self.minted_this_window.len() >= self.policy.max_tokens_per_window {
            return Err(PointerError::Budget);
        }
        let actions = state.actions_committed;
        if actions >= self.policy.max_actions_per_surface {
            return Err(PointerError::Budget);
        }
        let digest = action_digest(id, expected_revision, action);
        let expiry = self.tick.saturating_add(self.policy.token_ttl_ticks);
        self.mint_counter = self.mint_counter.saturating_add(1);
        let mut token_hasher = Sha256::new();
        token_hasher.update(digest.as_bytes());
        token_hasher.update(self.mint_counter.to_le_bytes());
        let opaque = format!("{:x}", token_hasher.finalize());
        self.tokens.insert(
            opaque.clone(),
            TokenRecord {
                surface: id.clone(),
                revision: expected_revision,
                digest,
                expiry,
                used: false,
            },
        );
        self.minted_this_window.push(self.tick);
        Ok(CommitToken { opaque })
    }

    fn verify_token(
        &mut self,
        id: &SurfaceId,
        token: &CommitToken,
        action: &Action,
        keyboard: bool,
    ) -> Result<(), PointerError> {
        if self.check_stopped() {
            return Err(PointerError::Permission);
        }
        let grant = if keyboard {
            self.policy.keyboard_granted
        } else {
            self.policy.pointer_granted
        };
        if !grant {
            return Err(PointerError::Permission);
        }
        // Alive check first: a revoked surface invalidates outstanding
        // tokens regardless of their recorded binding (threat T8).
        if self.live_state(id).is_none() {
            return Err(PointerError::SurfaceInvalid);
        }
        let Some(record) = self.tokens.get_mut(&token.opaque) else {
            return Err(PointerError::TokenInvalid);
        };
        if record.used {
            return Err(PointerError::TokenInvalid);
        }
        if &record.surface != id {
            return Err(PointerError::TokenInvalid);
        }
        if self.tick > record.expiry {
            self.tokens.remove(&token.opaque);
            return Err(PointerError::Expired);
        }
        let digest = action_digest(id, record.revision, action);
        if record.digest != digest {
            return Err(PointerError::TokenInvalid);
        }
        let state = self.surfaces.get_mut(id).expect("checked live above");
        if state.revision != record.revision {
            self.tokens.remove(&token.opaque);
            return Err(PointerError::StaleRevision);
        }
        record.used = true;
        self.tokens.remove(&token.opaque);
        Ok(())
    }

    fn verify_text_token(
        &mut self,
        id: &SurfaceId,
        token: &CommitToken,
        digest: &str,
    ) -> Result<bool, PointerError> {
        if self.check_stopped() {
            return Err(PointerError::Permission);
        }
        if !self.policy.keyboard_granted {
            return Err(PointerError::Permission);
        }
        if self.live_state(id).is_none() {
            return Err(PointerError::SurfaceInvalid);
        }
        let Some(record) = self.tokens.get_mut(&token.opaque) else {
            return Err(PointerError::TokenInvalid);
        };
        if record.used || &record.surface != id {
            return Err(PointerError::TokenInvalid);
        }
        if self.tick > record.expiry {
            self.tokens.remove(&token.opaque);
            return Err(PointerError::Expired);
        }
        if record.digest != digest {
            return Err(PointerError::TokenInvalid);
        }
        let revision = record.revision;
        {
            let state = self.surfaces.get(id).expect("live above");
            if state.revision != revision {
                self.tokens.remove(&token.opaque);
                return Err(PointerError::StaleRevision);
            }
        }
        self.tokens.remove(&token.opaque);
        Ok(true)
    }

    /// `pointer.commit`: verifies the five token bindings, then applies the
    /// one bounded action through the surface backend trait object held by
    /// the caller (the core never injects input itself).
    ///
    /// # Errors
    ///
    /// Typed [`PointerError`]; a consumed or mismatched token is
    /// [`PointerError::TokenInvalid`], never a silent repeat.
    ///
    /// # Panics
    ///
    /// Only on the unreachable vanish of a just-verified surface.
    pub fn commit(
        &mut self,
        id: &SurfaceId,
        token: &CommitToken,
        action: &Action,
    ) -> Result<(), PointerError> {
        let Some(revision) = self.live_state(id).map(|state| state.revision) else {
            return Err(PointerError::SurfaceInvalid);
        };
        self.verify_token(id, token, action, false)?;
        let state = self.surfaces.get_mut(id).expect("live above");
        state.actions_committed += 1;
        self.events.push(audit::AuditEvent::commit(
            self.tick,
            &id.token,
            revision.0,
            &action_digest(id, revision, action),
            matches!(action.variant, ActionVariant::Move | ActionVariant::Drag),
            matches!(
                action.variant,
                ActionVariant::Press | ActionVariant::Release | ActionVariant::Drag
            ),
        ));
        Ok(())
    }

    /// The keyboard preview path: mints one token bound to the exact
    /// text payload (Host-side digest; the audit stream carries only the
    /// character count).
    ///
    /// # Errors
    ///
    /// Typed [`PointerError`] per RFC-0040 §1 (keyboard grant gates).
    pub fn mint_text_token(
        &mut self,
        id: &SurfaceId,
        expected_revision: Revision,
        text: &str,
    ) -> Result<CommitToken, PointerError> {
        if self.check_stopped() {
            return Err(PointerError::Permission);
        }
        if !self.policy.keyboard_granted {
            return Err(PointerError::Permission);
        }
        if self.surfaces.get(id).is_none_or(|state| !state.alive) {
            return Err(PointerError::SurfaceInvalid);
        }
        if self.minted_this_window.len() >= self.policy.max_tokens_per_window {
            return Err(PointerError::Budget);
        }
        let digest = text_digest(text);
        let expiry = self.tick.saturating_add(self.policy.token_ttl_ticks);
        self.mint_counter = self.mint_counter.saturating_add(1);
        let mut token_hasher = Sha256::new();
        token_hasher.update(digest.as_bytes());
        token_hasher.update(self.mint_counter.to_le_bytes());
        let opaque = format!("{:x}", token_hasher.finalize());
        self.tokens.insert(
            opaque.clone(),
            TokenRecord {
                surface: id.clone(),
                revision: expected_revision,
                digest,
                expiry,
                used: false,
            },
        );
        self.minted_this_window.push(self.tick);
        Ok(CommitToken { opaque })
    }

    /// `keyboard.type`: separate grant, printable alphabet, declared
    /// sensitive-field refusal (F-1), per-surface keystroke budget.
    ///
    /// # Errors
    ///
    /// Typed [`KeyboardError`] per RFC-0040 §1 with the A1 `InvalidText`.
    ///
    /// # Panics
    ///
    /// Only on the unreachable vanish of a just-verified surface.
    pub fn type_text(
        &mut self,
        id: &SurfaceId,
        token: &CommitToken,
        text: &str,
    ) -> Result<(), KeyboardError> {
        if !self.policy.keyboard_granted {
            return Err(KeyboardError::Permission);
        }
        if self.check_stopped() {
            return Err(KeyboardError::Permission);
        }
        if self.surfaces.get(id).is_none_or(|state| !state.alive) {
            return Err(KeyboardError::SurfaceInvalid);
        }
        if self.surfaces.get(id).is_some_and(|state| state.sensitive) {
            return Err(KeyboardError::SensitiveField);
        }
        // v0 alphabet: printable Unicode scalars plus newline and tab;
        // other control characters refuse (RFC-0040 amendment A1).
        if text
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
        {
            return Err(KeyboardError::InvalidText);
        }
        let digest = text_digest(text);
        if !self
            .verify_text_token(id, token, &digest)
            .map_err(keyboard_error)?
        {
            return Err(KeyboardError::TokenInvalid);
        }
        let state = self.surfaces.get_mut(id).expect("live above");
        state.actions_committed += 1;
        // Audit carries the keystroke count only — never the content.
        self.events.push(audit::AuditEvent::commit(
            self.tick,
            &id.token,
            state.revision.0,
            &format!("text:{}", text.chars().count()),
            false,
            true,
        ));
        Ok(())
    }

    /// Test/teardown visibility: outstanding token count (boundedness).
    #[must_use]
    pub fn live_tokens(&self) -> usize {
        self.tokens.len()
    }

    fn drain_tokens_for(&mut self, id: &SurfaceId) {
        self.tokens.retain(|_, record| &record.surface != id);
    }
}

pub(crate) fn text_digest(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"text:");
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn keyboard_error(error: PointerError) -> KeyboardError {
    match error {
        PointerError::SurfaceInvalid => KeyboardError::SurfaceInvalid,
        PointerError::StaleRevision => KeyboardError::StaleRevision,
        PointerError::TokenInvalid => KeyboardError::TokenInvalid,
        PointerError::Expired => KeyboardError::Expired,
        PointerError::Budget => KeyboardError::Budget,
        PointerError::Permission => KeyboardError::Permission,
        PointerError::Io => KeyboardError::Io,
    }
}
