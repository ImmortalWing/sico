//! M18 pilot 3 — Web/UI application (M15 + RFC-0042): a log-viewer whose
//! business component (pure Script profile) renders the UI tree as JSON
//! and re-renders per typed event (event-in-arguments pattern). The
//! generated page embeds the compiled core module + RFC-0042 renderer and
//! simulates a deterministic event script in headless Edge.
//!
//! ```text
//! cargo test -p sico-cli --offline --test generate_web_ui_pilot -- --ignored --nocapture
//! ```

use std::{fs, path::PathBuf, process::Command};

use wasmparser::{Parser, Payload};

#[test]
#[ignore = "explicit fixture generator: refreshes web/log-viewer.html"]
fn generate_web_ui_pilot() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let web_dir = repository.join("web");
    fs::create_dir_all(&web_dir).unwrap();

    // Build the pure business component.
    let source = repository
        .join("pilots/web-log-viewer/log-viewer.sico")
        .display()
        .to_string();
    let component_path = web_dir.join("log-viewer.component.wasm");
    let _ = fs::remove_file(&component_path);
    let output = Command::new(env!("CARGO_BIN_EXE_sico"))
        .args([
            "build",
            "--profile",
            "script-v0",
            "--output",
            component_path.to_str().unwrap(),
            &source,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "component build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Extract the embedded core module.
    let component = fs::read(&component_path).unwrap();
    let mut core: Option<Vec<u8>> = None;
    for payload in Parser::new(0).parse_all(&component) {
        if let Payload::ModuleSection {
            unchecked_range, ..
        } = payload.unwrap()
        {
            core = Some(component[unchecked_range.start..unchecked_range.end].to_vec());
            break;
        }
    }
    let core = core.expect("component embeds a core module");
    let b64 = base64(&core);
    println!("core module: {} bytes", core.len());

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>Sico log viewer (M18 pilot 3)</title></head>
<body>
<h1>Sico log viewer</h1>
<div id="ui"></div>
<div id="results">RUNNING</div>
<script>
{shim}
</script>
<script>
const CORE_B64 = "{b64}";

// Event script (deterministic): the filter button toggles error-only
// filtering. Each event re-invokes the component with the event encoded
// in arguments (stateless component pattern, v0).
const EVENT_SCRIPT = [
  {{ kind: "click", node_id: "filter-errors" }},
  {{ kind: "click", node_id: "filter-all" }},
];

function b64ToBytes(b64) {{
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}}

(async () => {{
  try {{
    const core = b64ToBytes(CORE_B64);
    let lastTree = null;
    const trace = [];
    for (const event of [null, ...EVENT_SCRIPT]) {{
      const args = event
        ? [JSON.stringify(event)]
        : [];
      const out = await sicoScriptRun(core, args, new Uint8Array());
      if (!out.ok) throw new Error("component error: " + out.message);
      lastTree = JSON.parse(new TextDecoder().decode(out.stdout));
      trace.push({{ event, tree: lastTree }});
      SicoUI.render(document.getElementById("ui"), lastTree);
    }}
    const finalSerialized = SicoUI.serialize(
      document.getElementById("ui").firstChild);
    document.getElementById("results").textContent =
      "UI-PILOT " + JSON.stringify({{ trace, finalSerialized }});
  }} catch (error) {{
    document.getElementById("results").textContent =
      "PILOT-ERROR " + String(error);
  }}
}})();
</script>
</body>
</html>
"#,
        shim = ui_pilot_shim(),
        b64 = b64,
    );
    fs::write(web_dir.join("log-viewer.html"), html).unwrap();
    println!("web UI pilot page written");
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// The canonical-ABI shim (same transport as STEP-0156's harness shim).
fn ui_pilot_shim() -> &'static str {
    r#"
async function sicoScriptRun(coreBytes, args, stdin) {
  const module = await WebAssembly.instantiate(coreBytes, {});
  const e = module.instance.exports;
  const view = () => new DataView(e.memory.buffer);
  const raw = () => new Uint8Array(e.memory.buffer);
  const realloc = (align, size) => e.cabi_realloc(0, 0, align, size);

  const encoder = new TextEncoder();
  const encoded = args.map(a => encoder.encode(a));
  const argsArrayPtr = realloc(4, encoded.length * 8);
  encoded.forEach((bytes, i) => {
    const ptr = realloc(4, bytes.length);
    raw().set(bytes, ptr);
    view().setUint32(argsArrayPtr + i * 8, ptr, true);
    view().setUint32(argsArrayPtr + i * 8 + 4, bytes.length, true);
  });

  const stdinPtr = realloc(4, stdin.length);
  raw().set(stdin, stdinPtr);

  const resultPtr = e.run(argsArrayPtr, encoded.length, stdinPtr, stdin.length);
  const tag = view().getUint8(resultPtr);
  let out;
  if (tag === 0) {
    const p = resultPtr + 8;
    const sp = view().getUint32(p, true);
    const sl = view().getUint32(p + 4, true);
    out = {
      ok: true,
      stdout: raw().slice(sp, sp + sl),
      stderr: raw().slice(sp + 8, sp + 8 + view().getUint32(p + 12, true)),
      exit: Number(view().getBigInt64(p + 16, true)),
    };
  } else {
    const code = view().getUint8(resultPtr + 8);
    const msgPtr = view().getUint32(resultPtr + 12, true);
    const msgLen = view().getUint32(resultPtr + 16, true);
    const message = new TextDecoder("utf-8", { fatal: true }).decode(
      raw().slice(msgPtr, msgPtr + msgLen));
    out = { ok: false, code, message, stdout: new Uint8Array(0), stderr: new Uint8Array(0), exit: -1 };
  }
  e.cabi_post_run(resultPtr);
  return out;
}

// RFC-0042 v0 renderer (compact form of the web/ui-corpus.html renderer).
const SicoUI = (() => {
  const WIDGETS = new Set(["container", "text", "button", "list"]);
  function build(node) {
    if (!WIDGETS.has(node.widget)) throw new Error("E-ui-unknown-widget: " + node.widget);
    if (!node.id) throw new Error("E-ui-id-required");
    const el = document.createElement(
      node.widget === "container" ? "div" : node.widget === "text" ? "span" : node.widget === "list" ? "ul" : node.widget);
    el.dataset.widget = node.widget;
    el.dataset.id = node.id;
    const a = node.accessibility || {};
    el.setAttribute("role", a.role || ({ container: "group", text: "text", button: "button", list: "list" }[node.widget]));
    if (a.name) el.setAttribute("aria-label", a.name);
    if (a.focus_order) el.setAttribute("tabindex", String(a.focus_order));
    const p = node.properties || {};
    if (node.widget === "text" || node.widget === "button") {
      el.textContent = p.text ?? "";
    } else if (node.widget === "list") {
      for (const item of p.items || []) {
        const li = document.createElement("li");
        li.textContent = String(item);
        el.appendChild(li);
      }
    }
    if (node.widget === "container") {
      el.dataset.layout = node.layout || "stack";
      for (const child of node.children || []) el.appendChild(build(child));
    }
    return el;
  }
  function serialize(el) {
    const parts = [
      (el.dataset.widget || "listitem") + ":text \"" + el.textContent + "\"" +
        (el.getAttribute("aria-label") ? " \"" + el.getAttribute("aria-label") + "\"" : ""),
    ];
    for (const child of el.children) parts.push(serialize(child));
    return parts.join("|");
  }
  return { render: (m, t) => { m.replaceChildren(build(t)); return m.firstChild; }, serialize };
})();
"#
}
