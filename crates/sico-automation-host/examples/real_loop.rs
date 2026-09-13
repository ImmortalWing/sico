//! M16 real-path evidence driver (M16 plan §5.5): observe → capture →
//! preview → execute-one → verify against the real fixture window, with a
//! raw JSONL log. Dry-run by default: `--commit` is the explicit
//! one-action unlock; everything else is observation only.
//!
//! Usage:
//!   `cargo run -p sico-automation-host --example real_loop [-- --commit]`
//!
//! Exit 0 = the loop closed (verify matched); 1 = any typed failure.

use sha2::{Digest, Sha256};
use sico_automation_host::{
    Action, ActionVariant, AutomationHost, Revision, SurfaceId, policy::AutomationPolicy,
};

fn main() {
    let commit = std::env::args().any(|arg| arg == "--commit");
    match run(commit) {
        Ok(()) => std::process::exit(0),
        Err(error) => {
            eprintln!("real_loop: {error}");
            std::process::exit(1);
        }
    }
}

fn frame_digest(frame: (&u32, &u32, &u32, &Vec<u8>)) -> String {
    let mut hasher = Sha256::new();
    hasher.update(frame.3);
    format!("{:x}", hasher.finalize())
}

/// The increment button's client-space center (fixture determinism
/// contract: client rect (20,96)-(160,136)).
const INCREMENT_CENTER: (u32, u32) = (90, 116);

fn run(commit: bool) -> Result<(), String> {
    let mut evidence: Vec<serde_json::Value> = Vec::new();
    let mut record = |event: serde_json::Value| {
        eprintln!("{event}");
        evidence.push(event);
    };

    let window = sico_automation_host::win32::find_fixture().ok_or("fixture window not found")?;
    let (width, height) = window.client_size().ok_or("fixture geometry failed")?;
    record(serde_json::json!({
        "phase": "observe", "hwnd": window.hwnd, "client": [width, height],
        "window_rect": window.window_rect(),
    }));

    // The Host core owns every authority decision; the adapter executes.
    let policy = AutomationPolicy {
        capture_granted: true,
        pointer_granted: commit,
        keyboard_granted: false,
        ..AutomationPolicy::default()
    };
    let mut core = AutomationHost::new(policy);
    let surface: SurfaceId = core.expose_surface("SicoFixture", width, height, false);
    let revision: Revision = core
        .revision(&surface)
        .map_err(|e| format!("revision: {e:?}"))?;

    let before = window.capture()?;
    let before_digest = frame_digest((&width, &height, &before.2, &before.3));
    record(serde_json::json!({
        "phase": "capture-before", "digest": before_digest,
        "size": [before.0, before.1],
    }));

    let action = Action {
        variant: ActionVariant::Press,
        x: INCREMENT_CENTER.0,
        y: INCREMENT_CENTER.1,
        duration_ms: 10,
    };

    if !commit {
        // Dry-run: the Host withholds minting; nothing touches the desktop.
        let refusal = core.mint_token(&surface, revision, &action);
        record(serde_json::json!({
            "phase": "dry-run", "mint": format!("{refusal:?}"),
        }));
        write_log(&evidence)?;
        return Err("dry-run completed without commit (rerun with --commit)".into());
    }

    let _token = core
        .mint_token(&surface, revision, &action)
        .map_err(|e| format!("mint: {e:?}"))?;
    record(serde_json::json!({ "phase": "preview", "action": [action.x, action.y] }));

    window.commit_click(action.x, action.y)?;
    // The fixture repaints on WM_PAINT; give the loop one slice.
    std::thread::sleep(std::time::Duration::from_millis(250));

    let after = window.capture()?;
    let after_digest = frame_digest((&width, &height, &after.2, &after.3));
    record(serde_json::json!({
        "phase": "capture-after", "digest": after_digest,
        "changed": after_digest != before_digest,
    }));

    // Verify: the committed click must have changed the frame (the
    // counter text moved 0 -> 1).
    if after_digest == before_digest {
        record(serde_json::json!({ "phase": "verify", "result": "mismatch" }));
        write_log(&evidence)?;
        return Err("verify failed: frame unchanged after commit".into());
    }
    record(serde_json::json!({ "phase": "verify", "result": "matched" }));
    write_log(&evidence)
}

/// Raw evidence: `target/evidence/step-m16-real-loop/real-loop.jsonl`.
fn write_log(evidence: &[serde_json::Value]) -> Result<(), String> {
    let dir = std::path::Path::new("target/evidence/step-m16-real-loop");
    std::fs::create_dir_all(dir).map_err(|e| format!("evidence dir: {e}"))?;
    let mut text = String::new();
    for event in evidence {
        text.push_str(&serde_json::to_string(event).map_err(|e| e.to_string())?);
        text.push('\n');
    }
    std::fs::write(dir.join("real-loop.jsonl"), text).map_err(|e| format!("evidence write: {e}"))
}
