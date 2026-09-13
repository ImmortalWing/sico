//! M14 STEP-0136: source-level `http@0.2.0` emission end-to-end. The
//! fixture compiles `tests/end-to-end/script-http2-request.sico` — pure
//! Sico source calling `sico.http2.request` — into a Component that imports
//! `sico:script/http@0.2.0`, and runs it through the real runner against
//! the deterministic TLS fixture server: a granted endpoint round-trips
//! status and body; an ungranted endpoint surfaces the typed `permission`
//! case name without a single connection.

use sico_http_provider::streaming::{HttpFixtureServer, ResponsePlan};
use sico_runner::{
    CancelToken, FsGrants, HttpPolicy, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const HTTP2_SOURCE: &str = include_str!("../../../tests/end-to-end/script-http2-request.sico");

fn compile_source() -> Vec<u8> {
    // Both tests compile concurrently: a per-call counter keeps their
    // scratch directories disjoint (a shared PID directory raced).
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-step0136-http2-{}-{}",
        std::process::id(),
        CALL.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir(&directory).unwrap();
    let source_path = directory.join("http2-request.sico");
    let component_path = directory.join("http2-request.component.wasm");
    std::fs::write(&source_path, HTTP2_SOURCE).unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = sico_cli::run(
        [
            std::ffi::OsString::from("sico"),
            std::ffi::OsString::from("build"),
            std::ffi::OsString::from("--profile"),
            std::ffi::OsString::from("script-v0"),
            std::ffi::OsString::from("--output"),
            component_path.as_os_str().to_owned(),
            source_path.as_os_str().to_owned(),
        ],
        &mut std::io::empty(),
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let component = std::fs::read(&component_path).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    component
}

fn run_source(component: &[u8], url: &str, net: &NetGrants, policy: &HttpPolicy) -> RunOutcome {
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_policy(component, &FsGrants::default(), net, policy)
        .expect("source-emitted http2 component links");
    prepared
        .run(
            &ScriptInput {
                arguments: vec![url.to_owned()],
                stdin: Vec::new(),
            },
            &RunnerLimits::default(),
            &CancelToken::new(),
        )
        .expect("input bounds hold")
}

#[test]
fn source_emitted_http2_request_roundtrips() {
    let mut server = HttpFixtureServer::start(vec![ResponsePlan::Length {
        status: 200,
        body: b"source-http2".to_vec(),
    }]);
    let url = format!("https+private://localhost:{}", server.port);
    let mut net = NetGrants::default();
    net.grant_secure(&url).expect("fixture grant parses");
    let policy = HttpPolicy::with_trust_roots(server.ca_pem.clone());
    let component = compile_source();
    match run_source(&component, &url, &net, &policy) {
        RunOutcome::Output(output) => {
            assert_eq!(output.stdout, b"200:source-http2".to_vec());
            assert_eq!(output.exit_code, 0);
        }
        other => panic!("expected guest output, got {other:?}"),
    }
    assert_eq!(server.request_count(), 1);
    server.stop();
}

#[test]
fn source_emitted_http2_typed_refusal_names_the_case() {
    let mut server = HttpFixtureServer::start(vec![ResponsePlan::Length {
        status: 200,
        body: b"must-not-arrive".to_vec(),
    }]);
    let url = format!("https+private://localhost:{}", server.port);
    let policy = HttpPolicy::with_trust_roots(server.ca_pem.clone());
    let component = compile_source();
    match run_source(&component, &url, &NetGrants::default(), &policy) {
        RunOutcome::Output(output) => {
            assert_eq!(
                output.stdout,
                b"error:permission".to_vec(),
                "the typed http-error case name crosses as source-level Text"
            );
            assert_eq!(output.exit_code, 1);
        }
        other => panic!("expected guest output, got {other:?}"),
    }
    assert_eq!(
        server.connection_count(),
        0,
        "no bytes on a denied endpoint"
    );
    server.stop();
}
