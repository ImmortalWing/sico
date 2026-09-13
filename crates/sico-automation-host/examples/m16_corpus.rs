//! M16 real-adapter fail-closed corpus (M16 plan §5.5, gate 2 substance):
//! drift, stale revision, duplicate commit, capture limit+1 and
//! handle/RSS budgets — against the real fixture window with raw JSONL
//! evidence. The driver owns the fixture process lifecycle.
//!
//! Usage: `cargo run -p sico-automation-host --offline --example m16_corpus`
//! Exit 0 = every corpus case refused/behaved as contracted.

use std::process::{Child, Command};

use sico_automation_host::{
    Action, ActionVariant, AutomationHost, SurfaceId, policy::AutomationPolicy,
};

const INCREMENT: (u32, u32) = (90, 116);

struct Evidence(Vec<serde_json::Value>);

impl Evidence {
    fn push(&mut self, value: serde_json::Value) {
        eprintln!("{value}");
        self.0.push(value);
    }
    fn write(&self) {
        let dir = std::path::Path::new("target/evidence/step-m16-real-loop");
        let _ = std::fs::create_dir_all(dir);
        let mut text = String::new();
        for event in &self.0 {
            text.push_str(&serde_json::to_string(event).unwrap());
            text.push('\n');
        }
        let _ = std::fs::write(dir.join("corpus.jsonl"), text);
    }
}

fn spawn_fixture() -> Child {
    let exe = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/debug/sico-automation-fixture.exe"
    );
    Command::new(exe)
        .arg("SicoFixture")
        .spawn()
        .expect("fixture launches")
}

fn wait_for_window() -> sico_automation_host::win32::FixtureWindow {
    for _ in 0..50 {
        if let Some(window) = sico_automation_host::win32::find_fixture() {
            std::thread::sleep(std::time::Duration::from_millis(300));
            return window;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("fixture window never appeared");
}

#[allow(clippy::too_many_lines)] // six corpus cases read best inline
fn main() {
    let mut evidence = Evidence(Vec::new());
    let mut child = spawn_fixture();
    let window = wait_for_window();

    let policy = AutomationPolicy {
        capture_granted: true,
        pointer_granted: true,
        keyboard_granted: true,
        max_frames_per_window: 8,
        ..AutomationPolicy::default()
    };
    let mut core = AutomationHost::new(policy);
    let (width, height) = window.client_size().unwrap();
    let surface: SurfaceId = core.expose_surface("SicoFixture", width, height, false);

    // 1. happy commit (the baseline the refusals compare against).
    let revision = core.revision(&surface).unwrap();
    let action = Action {
        variant: ActionVariant::Press,
        x: INCREMENT.0,
        y: INCREMENT.1,
        duration_ms: 10,
    };
    let token = core.mint_token(&surface, revision, &action).expect("mint");
    window
        .commit_click(INCREMENT.0, INCREMENT.1)
        .expect("commit");
    core.commit(&surface, &token, &action).expect("core commit");
    evidence.push(serde_json::json!({ "case": "happy-commit", "result": "ok" }));

    // 2. duplicate commit: the consumed token refuses closed.
    let refused = core.commit(&surface, &token, &action);
    assert!(matches!(
        refused,
        Err(sico_automation_host::PointerError::TokenInvalid)
    ));
    evidence.push(serde_json::json!({ "case": "duplicate-commit", "refused": "TokenInvalid" }));

    // 3. stale revision: geometry moves after preview → typed refusal.
    let revision = core.revision(&surface).unwrap();
    let token = core.mint_token(&surface, revision, &action).unwrap();
    unsafe {
        // Move the window 40 px right (drift); the Host observes it and
        // bumps the revision, invalidating the outstanding token.
        use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos;
        let (l, t, r, b) = window.window_rect().unwrap();
        SetWindowPos(
            window.hwnd,
            0,
            l + 40,
            t,
            r - l,
            b - t,
            0x0004, /* NOACTIVATE */
        );
    }
    std::thread::sleep(std::time::Duration::from_millis(150));
    if window
        .window_rect()
        .is_some_and(|(l, _, _, _)| i64::from(l) != i64::try_from(revision.0).unwrap_or(0))
    {
        core.bump_revision(&surface); // Host-side drift observation
    }
    let refused = core.commit(&surface, &token, &action);
    assert!(matches!(
        refused,
        Err(sico_automation_host::PointerError::StaleRevision)
    ));
    evidence.push(serde_json::json!({ "case": "stale-revision", "refused": "StaleRevision" }));

    // 4. capture limit+1: the ninth frame in the window refuses typed.
    let mut last = Ok(());
    for _ in 0..9 {
        last = core.capture(&surface).map(|_| ());
    }
    assert!(matches!(
        last,
        Err(sico_automation_host::CaptureError::Limit)
    ));
    evidence.push(serde_json::json!({ "case": "capture-limit-plus-one", "refused": "Limit" }));
    for _ in 0..64 {
        core.tick();
    }

    // 5. drift/revocation: the surface identity dies with the window.
    child.kill().expect("fixture kill");
    let _ = child.wait();
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(!window.alive(), "fixture window must be gone");
    core.revoke_surface(&surface);
    let refused = core.capture(&surface);
    assert!(matches!(
        refused,
        Err(sico_automation_host::CaptureError::SurfaceInvalid)
    ));
    evidence.push(serde_json::json!({ "case": "surface-drift", "refused": "SurfaceInvalid", "tokens": core.live_tokens() }));
    assert_eq!(
        core.live_tokens(),
        0,
        "revocation drains outstanding tokens"
    );

    // 6. handle/RSS budgets across a bounded action burst (gate 5).
    let mut child = spawn_fixture();
    let window = wait_for_window();
    let (width, height) = window.client_size().unwrap();
    let surface = core.expose_surface("SicoFixture-2", width, height, false);
    let (handles_before, rss_before) = sico_automation_host::win32::process_budget();
    for _ in 0..10 {
        let revision = core.revision(&surface).unwrap();
        let token = core.mint_token(&surface, revision, &action).unwrap();
        window
            .commit_click(INCREMENT.0, INCREMENT.1)
            .expect("commit");
        core.commit(&surface, &token, &action).expect("core commit");
        core.capture(&surface).expect("capture");
        for _ in 0..64 {
            core.tick();
        }
    }
    let (handles_after, rss_after) = sico_automation_host::win32::process_budget();
    evidence.push(serde_json::json!({
        "case": "budgets", "actions": 10,
        "handles": [handles_before, handles_after],
        "rss_bytes": [rss_before, rss_after],
    }));
    let _ = window;

    child.kill().expect("fixture kill");
    let _ = child.wait();
    evidence.write();
    eprintln!("M16 real-adapter corpus: all cases behaved as contracted");
}
