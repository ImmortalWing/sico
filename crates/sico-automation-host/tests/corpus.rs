//! The fail-closed corpus for the RFC-0040 §4 refusal rows, frozen against
//! the Host core with the synthetic surface (`contract-verified`).

use sico_automation_host::policy::AutomationPolicy;
use sico_automation_host::{
    Action, ActionVariant, AutomationHost, KeyboardError, PointerError, SurfaceId,
};

fn press(x: u32, y: u32) -> Action {
    Action {
        variant: ActionVariant::Press,
        x,
        y,
        duration_ms: 10,
    }
}

fn host() -> (AutomationHost, SurfaceId) {
    let mut core = AutomationHost::new(AutomationPolicy {
        keyboard_granted: true,
        ..AutomationPolicy::default()
    });
    let id = core.expose_surface("fixture-main", 640, 480, false);
    (core, id)
}

#[test]
fn commit_without_token_refuses() {
    let (mut core, id) = host();
    let error = core.commit(
        &id,
        &sico_automation_host::CommitToken {
            opaque: "ghost".into(),
        },
        &press(10, 10),
    );
    assert_eq!(error, Err(PointerError::TokenInvalid));
}

#[test]
fn expired_token_refuses() {
    let (mut core, id) = host();
    let revision = core.revision(&id).unwrap();
    let token = core.mint_token(&id, revision, &press(10, 10)).unwrap();
    for _ in 0..20 {
        core.tick();
    }
    assert_eq!(
        core.commit(&id, &token, &press(10, 10)),
        Err(PointerError::Expired)
    );
}

#[test]
fn reused_token_refuses() {
    let (mut core, id) = host();
    let revision = core.revision(&id).unwrap();
    let token = core.mint_token(&id, revision, &press(10, 10)).unwrap();
    assert_eq!(core.commit(&id, &token, &press(10, 10)), Ok(()));
    assert_eq!(
        core.commit(&id, &token, &press(10, 10)),
        Err(PointerError::TokenInvalid)
    );
}

#[test]
fn stale_revision_refuses() {
    let (mut core, id) = host();
    let revision = core.revision(&id).unwrap();
    let token = core.mint_token(&id, revision, &press(10, 10)).unwrap();
    // The surface moved between preview and commit: the revision moved.
    core.bump_revision(&id);
    assert_eq!(
        core.commit(&id, &token, &press(10, 10)),
        Err(PointerError::StaleRevision)
    );
}

#[test]
fn surface_drift_invalidates_identity() {
    let (mut core, id) = host();
    let revision = core.revision(&id).unwrap();
    let token = core.mint_token(&id, revision, &press(10, 10)).unwrap();
    core.revoke_surface(&id);
    assert_eq!(
        core.commit(&id, &token, &press(10, 10)),
        Err(PointerError::SurfaceInvalid)
    );
    assert_eq!(
        core.revision(&id),
        Err(sico_automation_host::ObserveError::SurfaceInvalid)
    );
    assert_eq!(
        core.live_tokens(),
        0,
        "revocation drains outstanding tokens"
    );
}

#[test]
fn duplicate_commit_is_a_reuse_refusal() {
    let (mut core, id) = host();
    let revision = core.revision(&id).unwrap();
    // Two identical previews are two distinct single-use tokens.
    let first = core.mint_token(&id, revision, &press(10, 10)).unwrap();
    let second = core.mint_token(&id, revision, &press(10, 10)).unwrap();
    assert_ne!(first, second, "identical previews never collide");
    assert_eq!(core.commit(&id, &first, &press(10, 10)), Ok(()));
    assert_eq!(core.commit(&id, &second, &press(10, 10)), Ok(()));
    // Re-committing the first token again is the reuse refusal.
    assert_eq!(
        core.commit(&id, &first, &press(10, 10)),
        Err(PointerError::TokenInvalid)
    );
}

#[test]
fn action_digest_binds_the_commit() {
    let (mut core, id) = host();
    let revision = core.revision(&id).unwrap();
    let token = core.mint_token(&id, revision, &press(10, 10)).unwrap();
    // A different action than previewed must not pass, even fresh.
    assert_eq!(
        core.commit(&id, &token, &press(11, 10)),
        Err(PointerError::TokenInvalid)
    );
}

#[test]
fn capture_does_not_imply_pointer() {
    let mut core = AutomationHost::new(AutomationPolicy {
        capture_granted: true,
        pointer_granted: false,
        keyboard_granted: false,
        ..AutomationPolicy::default()
    });
    let id = core.expose_surface("capture-only", 640, 480, false);
    assert!(core.capture(&id).is_ok(), "capture grant alone captures");
    let revision = core.revision(&id).unwrap();
    assert_eq!(
        core.mint_token(&id, revision, &press(1, 1)),
        Err(PointerError::Permission),
        "capture must not widen into input"
    );
}

#[test]
fn pointer_does_not_imply_keyboard() {
    let mut core = AutomationHost::new(AutomationPolicy {
        capture_granted: false,
        pointer_granted: true,
        keyboard_granted: false,
        ..AutomationPolicy::default()
    });
    let id = core.expose_surface("pointer-only", 640, 480, false);
    let revision = core.revision(&id).unwrap();
    let token = core.mint_token(&id, revision, &press(1, 1)).unwrap();
    assert_eq!(
        core.type_text(&id, &token, "hi"),
        Err(KeyboardError::Permission),
        "keyboard is a separate grant"
    );
}

#[test]
fn sensitive_field_refuses_closed() {
    let (mut core, id) = core_with_sensitive();
    let revision = core.revision(&id).unwrap();
    let token = core.mint_token(&id, revision, &press(5, 5)).unwrap();
    assert_eq!(
        core.type_text(&id, &token, "secret"),
        Err(KeyboardError::SensitiveField)
    );
}

fn core_with_sensitive() -> (AutomationHost, SurfaceId) {
    let mut core = AutomationHost::new(AutomationPolicy {
        keyboard_granted: true,
        ..AutomationPolicy::default()
    });
    let id = core.expose_surface("password-dialog", 640, 480, true);
    (core, id)
}

#[test]
fn non_printable_text_refuses() {
    let (mut core, id) = host();
    let revision = core.revision(&id).unwrap();
    let token = core.mint_text_token(&id, revision, "a\u{0}b").unwrap();
    assert_eq!(
        core.type_text(&id, &token, "a\u{0}b"),
        Err(KeyboardError::InvalidText)
    );
    let token = core
        .mint_text_token(&id, revision, "line\nbreak\tok")
        .unwrap();
    assert_eq!(core.type_text(&id, &token, "line\nbreak\tok"), Ok(()));
}

#[test]
fn limit_plus_one_for_rate_and_budgets() {
    // Token rate: one past the per-window ceiling refuses with Budget.
    let (mut core, id) = host();
    let revision = core.revision(&id).unwrap();
    let mut last = None;
    for _ in 0..32 {
        last = Some(core.mint_token(&id, revision, &press(1, 1)));
    }
    assert!(last.expect("32 mints within window").is_ok());
    assert_eq!(
        core.mint_token(&id, revision, &press(1, 1)),
        Err(PointerError::Budget),
        "token 33 in the window is limit+1"
    );
    for _ in 0..64 {
        core.tick();
    }
    assert!(
        core.mint_token(&id, revision, &press(1, 1)).is_ok(),
        "window slides"
    );

    // Capture rate: one past the frame ceiling refuses with Limit.
    let (mut core, id) = host();
    let mut last = None;
    for _ in 0..60 {
        last = Some(core.capture(&id));
    }
    assert!(last.expect("60 frames within window").is_ok());
    assert!(matches!(
        core.capture(&id),
        Err(sico_automation_host::CaptureError::Limit)
    ));
}

#[test]
fn dry_run_withholds_minting_guest_invisibly() {
    let mut core = AutomationHost::new(AutomationPolicy {
        capture_granted: true,
        pointer_granted: false,
        keyboard_granted: false,
        ..AutomationPolicy::default()
    });
    // Dry-run is enforced by not granting the input authority to the run
    // at all (ADR-0013 §4: the Host withholds minting; the guest cannot
    // distinguish or disable it because the mode is never guest-visible).
    let id = core.expose_surface("dry-run-fixture", 640, 480, false);
    let revision = core.revision(&id).unwrap();
    assert_eq!(
        core.mint_token(&id, revision, &press(1, 1)),
        Err(PointerError::Permission)
    );
    assert!(
        core.capture(&id).is_ok(),
        "observe-side capabilities keep working"
    );
}

#[test]
fn emergency_stop_suppresses_everything() {
    let (mut core, id) = host();
    let revision = core.revision(&id).unwrap();
    let token = core.mint_token(&id, revision, &press(10, 10)).unwrap();
    core.emergency_stop();
    assert_eq!(
        core.commit(&id, &token, &press(10, 10)),
        Err(PointerError::Permission)
    );
    assert!(matches!(
        core.capture(&id),
        Err(sico_automation_host::CaptureError::Permission)
    ));
    assert_eq!(core.live_tokens(), 0, "stop drains outstanding tokens");
    let events = core.audit();
    assert_eq!(events.last().expect("stop is audited").class, "stop");
}

#[test]
fn audit_stream_is_append_only_and_redacted() {
    let (mut core, id) = host();
    let revision = core.revision(&id).unwrap();
    let token = core.mint_token(&id, revision, &press(10, 10)).unwrap();
    core.commit(&id, &token, &press(10, 10)).unwrap();
    let mut lines = core
        .audit()
        .iter()
        .map(sico_automation_host::audit::AuditEvent::jsonl)
        .collect::<Vec<_>>();
    assert!(!lines.is_empty());
    for line in &lines {
        assert!(
            !line.contains("pixels") && !line.contains("keystroke"),
            "{line}"
        );
        assert!(line.contains("\"class\""), "{line}");
    }
    let before = lines.join("\n");
    let revision = core.revision(&id).unwrap();
    let token = core.mint_token(&id, revision, &press(20, 20)).unwrap();
    core.commit(&id, &token, &press(20, 20)).unwrap();
    lines = core
        .audit()
        .iter()
        .map(sico_automation_host::audit::AuditEvent::jsonl)
        .collect::<Vec<_>>();
    assert!(
        lines.join("\n").starts_with(&before),
        "append-only: earlier lines unchanged"
    );
}
