//! M18 pilot 1 — secure API agent (M12): a Sico source program performs a
//! three-step agent flow (config → task → result POST) against the TLS
//! fixture server over `sico.http2.request`, with fail-closed typed
//! errors. Raw evidence: stdout byte-exact, sequential response plans.

use sico_http_provider::streaming::{HttpFixtureServer, ResponsePlan};
use sico_runner::{
    CancelToken, FsGrants, HttpPolicy, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

fn compile_pilot() -> Vec<u8> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALL: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "sico-m18-api-agent-{}-{}",
        std::process::id(),
        CALL.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("api-agent.sico");
    let component_path = directory.join("api-agent.component.wasm");
    std::fs::write(
        &source_path,
        include_str!("../../../pilots/api-agent/api-agent.sico"),
    )
    .unwrap();
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

fn run_agent(component: &[u8], url: &str, net: &NetGrants, policy: &HttpPolicy) -> RunOutcome {
    let runner = Runner::new().expect("runner builds");
    let prepared = runner
        .prepare_program_with_policy(component, &FsGrants::default(), net, policy)
        .expect("agent component links");
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
fn api_agent_pilot_completes_three_step_flow() {
    let server = HttpFixtureServer::start(vec![
        ResponsePlan::Length {
            status: 200,
            body: b"mode=strict\nworkers=2".to_vec(),
        },
        ResponsePlan::Length {
            status: 200,
            body: b"task-7:summarize:alpha-log".to_vec(),
        },
        ResponsePlan::Length {
            status: 200,
            body: b"accepted".to_vec(),
        },
    ]);
    let url = format!("https+private://localhost:{}", server.port);
    let mut net = NetGrants::default();
    net.grant_secure(&url).expect("fixture grant parses");
    let policy = HttpPolicy::with_trust_roots(server.ca_pem.clone());
    let component = compile_pilot();
    match run_agent(&component, &url, &net, &policy) {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 0);
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                "agent:config-ok|task-ok|result-accepted"
            );
            assert_eq!(String::from_utf8_lossy(&output.stderr), "");
        }
        other => panic!("unexpected outcome: {other:?}"),
    }
}

#[test]
fn api_agent_pilot_fails_closed_on_unexpected_config() {
    let server = HttpFixtureServer::start(vec![ResponsePlan::Length {
        status: 200,
        body: b"mode=permissive".to_vec(),
    }]);
    let url = format!("https+private://localhost:{}", server.port);
    let mut net = NetGrants::default();
    net.grant_secure(&url).expect("fixture grant parses");
    let policy = HttpPolicy::with_trust_roots(server.ca_pem.clone());
    let component = compile_pilot();
    match run_agent(&component, &url, &net, &policy) {
        RunOutcome::Output(output) => {
            assert_eq!(output.exit_code, 1);
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                "agent-fail:unexpected config"
            );
        }
        other => panic!("unexpected outcome: {other:?}"),
    }
}
