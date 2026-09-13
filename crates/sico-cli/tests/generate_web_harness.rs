//! STEP-0156 fixture generator: builds the cross-host business component
//! and emits the self-contained M15 web-harness page (JS canonical-ABI
//! shim + base64-embedded core module + fixture cases). Run explicitly:
//!
//! ```text
//! cargo test -p sico-cli --offline --test generate_web_harness -- --ignored --nocapture
//! ```

use std::{fs, path::PathBuf, process::Command};

use wasmparser::{Parser, Payload};

#[test]
#[ignore = "explicit fixture generator: refreshes web/harness.html"]
fn generate_web_harness() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let web_dir = repository.join("web");
    fs::create_dir_all(&web_dir).unwrap();

    // 1. Build the pure business component (script profile).
    let source = repository
        .join("tests/end-to-end/script-word-count.sico")
        .display()
        .to_string();
    let component_path = web_dir.join("guest.wordcount.component.wasm");
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

    // 2. Extract the embedded core module (the component wraps it; the
    //    JS shim executes the core module directly — ADR-0014).
    let component = fs::read(&component_path).unwrap();
    let mut core: Option<Vec<u8>> = None;
    for payload in Parser::new(0).parse_all(&component) {
        if let Payload::ModuleSection {
            unchecked_range, ..
        } = payload.unwrap()
        {
            core = Some(component[unchecked_range.start..unchecked_range.end].to_vec());
            break; // the script world wraps exactly one core module
        }
    }
    let core = core.expect("component embeds a core module");
    let core_path = web_dir.join("guest.wordcount.core.wasm");
    fs::write(&core_path, &core).unwrap();
    println!("core module: {} bytes", core.len());

    // 3. Emit the self-contained harness page (base64-embedded core).
    let b64 = base64(&core);
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>Sico web host harness</title></head>
<body>
<div id="results">RUNNING</div>
<script>
{shim}
</script>
<script>
const CORE_B64 = "{b64}";
const CASES = [
  {{ name: "two-words", args: [], stdin: new TextEncoder().encode("alpha beta") }},
  {{ name: "trim-and-punct", args: [], stdin: new TextEncoder().encode("  one, two; three.  ") }},
  {{ name: "empty", args: [], stdin: new TextEncoder().encode("") }},
  {{ name: "one-word", args: [], stdin: new TextEncoder().encode("solo") }},
  {{ name: "utf8", args: [], stdin: new TextEncoder().encode(" café roi ") }},
];

function b64ToBytes(b64) {{
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}}

(async () => {{
  const results = [];
  try {{
    const core = b64ToBytes(CORE_B64);
    for (const c of CASES) {{
      const out = await sicoScriptRun(core, c.args, c.stdin);
      results.push({{
        name: c.name,
        exit: out.exit,
        stdoutHex: Array.from(out.stdout).map(b => b.toString(16).padStart(2, "0")).join(""),
        stdoutText: new TextDecoder().decode(out.stdout),
      }});
    }}
  }} catch (error) {{
    document.getElementById("results").textContent =
      "HARNESS-ERROR " + String(error);
    return;
  }}
  document.getElementById("results").textContent =
    "HARNESS-RESULTS " + JSON.stringify(results);
}})();
</script>
</body>
</html>
"#,
        shim = sico_script_shim(),
        b64 = b64,
    );
    fs::write(web_dir.join("harness.html"), html).unwrap();
    println!("harness page written ({} bytes core)", core.len());
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

/// The JS canonical-ABI shim for the frozen Script v0 world, injected
/// verbatim into the harness page. Kept in Rust as one string so the
/// generator is self-contained.
fn sico_script_shim() -> &'static str {
    r#"
// ADR-0014 substrate: canonical-ABI transport over the embedded core
// module. Frozen layout: run(argsPtr,argsLen,stdinPtr,stdinLen) -> ptr
// to 32-byte result area (tag u8 @0, payload @8); script-output 24B
// (stdout.ptr/len @0/4, stderr @8/12, exit_code s64 @16); script-error
// 12B (code u8 @0, message.ptr/len @4/8). All pointers into the single
// guest linear memory; cabi_realloc grows; cabi_post_run frees.
async function sicoScriptRun(coreBytes, args, stdin) {
  const module = await WebAssembly.instantiate(coreBytes, {});
  const e = module.instance.exports;
  const memory = new DataView(e.memory.buffer);
  const bytes = new Uint8Array(e.memory.buffer);
  const realloc = (align, size) => e.cabi_realloc(0, 0, align, size);

  // Lower arguments: array of (ptr,len) pairs + UTF-8 bytes each.
  const encoder = new TextEncoder();
  const encoded = args.map(a => encoder.encode(a));
  const argsArrayPtr = realloc(4, encoded.length * 8);
  encoded.forEach((bytes, i) => {
    const ptr = realloc(4, bytes.length);
    bytes.forEach((b, j) => { bytes0()[ptr + j] = b; });
    memory.setUint32(argsArrayPtr + i * 8, ptr, true);
    memory.setUint32(argsArrayPtr + i * 8 + 4, bytes.length, true);
  });
  function bytes0() { return new Uint8Array(e.memory.buffer); }

  const stdinPtr = realloc(4, stdin.length);
  new Uint8Array(e.memory.buffer).set(stdin, stdinPtr);

  const resultPtr = e.run(argsArrayPtr, encoded.length, stdinPtr, stdin.length);
  const tag = memory.getUint8(resultPtr);
  let out;
  if (tag === 0) {
    const p = resultPtr + 8;
    out = {
      ok: true,
      stdout: bytes.slice(memory.getUint32(p, true),
                          memory.getUint32(p, true) + memory.getUint32(p + 4, true)),
      stderr: bytes.slice(memory.getUint32(p + 8, true),
                          memory.getUint32(p + 8, true) + memory.getUint32(p + 12, true)),
      exit: Number(memory.getBigInt64(p + 16, true)),
    };
  } else {
    const code = memory.getUint8(resultPtr + 8);
    const msgPtr = memory.getUint32(resultPtr + 12, true);
    const msgLen = memory.getUint32(resultPtr + 16, true);
    // Strict UTF-8 fail-closed (frozen layout: Text validated at the lift).
    const message = new TextDecoder("utf-8", { fatal: true }).decode(
      bytes.slice(msgPtr, msgPtr + msgLen));
    out = { ok: false, code, message, stdout: new Uint8Array(0), stderr: new Uint8Array(0), exit: -1 };
  }
  e.cabi_post_run(resultPtr);
  return out;
}
"#
}
