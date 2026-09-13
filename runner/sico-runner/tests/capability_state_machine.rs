//! M14 STEP-0139: the capability-backed state machine acceptance
//! application (`tests/end-to-end/capability-state-machine.sico`) through
//! the real Component Runtime. The machine persists a flat-JSON state
//! document through the granted scoped `sico.fs.write` capability (the
//! explicit external effect), guards every transition against the caller's
//! expected revision (unary counter), refuses illegal transitions with
//! typed `DomainError`s, and recovers deterministically from a corrupted
//! or missing state document.

use sico_runner::{
    CancelToken, FsGrants, NetGrants, RunOutcome, Runner, RunnerLimits, ScriptInput,
};

const MACHINE_SOURCE: &str =
    include_str!("../../../tests/end-to-end/capability-state-machine.sico");

struct Machine {
    root: std::path::PathBuf,
}

impl Machine {
    fn new(tag: u32) -> Self {
        let root =
            std::env::temp_dir().join(format!("sico-step0139-csm-{}-{tag}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn component(&self) -> Vec<u8> {
        let source_path = self.root.join("capability-state-machine.sico");
        let component_path = self.root.join("capability-state-machine.component.wasm");
        std::fs::write(&source_path, MACHINE_SOURCE).unwrap();
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
        std::fs::read(&component_path).unwrap()
    }

    fn grants(&self) -> FsGrants {
        // The runner's contract: grant roots are canonicalized by the caller.
        let canonical = self.root.canonicalize().unwrap();
        FsGrants {
            read_roots: vec![canonical.clone()],
            write_roots: vec![canonical],
        }
    }

    fn run(&self, component: &[u8], command: &str, expected: &str) -> RunOutcome {
        let runner = Runner::new().expect("runner builds");
        let prepared = runner
            .prepare_program_with_net(component, &self.grants(), &NetGrants::default())
            .expect("state machine component links");
        prepared
            .run(
                &ScriptInput {
                    arguments: vec![command.to_owned(), expected.to_owned()],
                    ..ScriptInput::default()
                },
                &RunnerLimits::default(),
                &CancelToken::new(),
            )
            .expect("input bounds hold")
    }

    fn stdout_of(outcome: RunOutcome) -> String {
        match outcome {
            RunOutcome::Output(output) => {
                assert_eq!(output.exit_code, 0);
                String::from_utf8(output.stdout).unwrap()
            }
            other => panic!("expected guest output, got {other:?}"),
        }
    }

    fn corrupt(&self) {
        std::fs::write(self.root.join("state.json"), b"{{{ not json").unwrap();
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn capability_state_machine_transitions_guards_and_recovers() {
    let machine = Machine::new(1);
    let component = machine.component();

    // Missing state: deterministic recovery to the initial document, then
    // the command applies against the recovered state.
    assert_eq!(
        Machine::stdout_of(machine.run(&component, "query", "")),
        "ok:sealed:0"
    );
    // Legal transition, guarded by the caller's expected revision.
    assert_eq!(
        Machine::stdout_of(machine.run(&component, "open", "")),
        "ok:open:1"
    );
    assert_eq!(
        Machine::stdout_of(machine.run(&component, "seal", "+")),
        "ok:sealed:2"
    );
    // Illegal transition: typed refusal, state document untouched.
    match machine.run(&component, "burn", "++") {
        RunOutcome::Domain { message, .. } => assert_eq!(message, "burn requires open"),
        other => panic!("expected typed refusal, got {other:?}"),
    }
    assert_eq!(
        Machine::stdout_of(machine.run(&component, "query", "++")),
        "ok:sealed:2"
    );
    // Stale revision guard: the caller's expectation lags the document.
    match machine.run(&component, "open", "+") {
        RunOutcome::Domain { message, .. } => {
            assert!(message.starts_with("stale revision"), "{message}")
        }
        other => panic!("expected typed stale refusal, got {other:?}"),
    }
    // Corrupted document: deterministic recovery back to the initial state.
    machine.corrupt();
    assert_eq!(
        Machine::stdout_of(machine.run(&component, "query", "")),
        "ok:sealed:0"
    );
    // The recovered machine accepts transitions again.
    assert_eq!(
        Machine::stdout_of(machine.run(&component, "open", "")),
        "ok:open:1"
    );
}

#[test]
fn repeated_state_machine_runs_are_deterministic() {
    let machine = Machine::new(2);
    let component = machine.component();
    for _ in 0..2 {
        assert_eq!(
            Machine::stdout_of(machine.run(&component, "query", "")),
            "ok:sealed:0"
        );
        assert_eq!(
            Machine::stdout_of(machine.run(&component, "open", "")),
            "ok:open:1"
        );
        // Reset for the next repetition.
        machine.corrupt();
    }
}
