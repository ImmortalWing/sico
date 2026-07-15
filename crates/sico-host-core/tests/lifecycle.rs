use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use sico_host_core::{
    CommandSpec, LaunchOutcome, LifecycleError, OpenRequest, ProcessSupervisor, TerminalOutcome,
};

static TEST_ID: AtomicU64 = AtomicU64::new(0);
const APP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn one_process_per_identity_and_bounded_open_queue() {
    let root = test_directory();
    let mut supervisor = ProcessSupervisor::open(&root).unwrap();
    let slow = slow_command();
    assert_eq!(
        supervisor.launch(APP, &slow).unwrap(),
        LaunchOutcome::Started
    );
    assert_eq!(
        supervisor.launch(APP, &slow).unwrap(),
        LaunchOutcome::AlreadyRunning
    );
    for index in 0..256 {
        supervisor
            .queue_open(
                APP,
                OpenRequest::new(PathBuf::from(format!("app-{index}.sapp"))).unwrap(),
            )
            .unwrap();
    }
    assert!(matches!(
        supervisor.queue_open(
            APP,
            OpenRequest::new(PathBuf::from("overflow.sapp")).unwrap()
        ),
        Err(LifecycleError::EventQueueFull)
    ));
    assert!(supervisor.take_open(APP).unwrap().is_some());
    assert_eq!(supervisor.cancel(APP).unwrap(), TerminalOutcome::Cancelled);
    assert!(!supervisor.is_running(APP));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn timeout_and_crash_do_not_prevent_healthy_relaunch() {
    let root = test_directory();
    let mut supervisor = ProcessSupervisor::open(&root).unwrap();
    supervisor.launch(APP, &slow_command()).unwrap();
    assert_eq!(
        supervisor
            .wait_with_timeout(APP, Duration::from_millis(30))
            .unwrap(),
        TerminalOutcome::TimedOut
    );
    supervisor.launch(APP, &exit_command(7)).unwrap();
    assert!(matches!(
        supervisor
            .wait_with_timeout(APP, Duration::from_secs(2))
            .unwrap(),
        TerminalOutcome::Crashed(Some(7))
    ));
    supervisor.launch(APP, &exit_command(0)).unwrap();
    assert!(matches!(
        supervisor
            .wait_with_timeout(APP, Duration::from_secs(2))
            .unwrap(),
        TerminalOutcome::Exited(Some(0))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn terminal_cleanup_removes_only_registered_host_file() {
    let root = test_directory();
    let cleanup = root.join("guest.tmp");
    fs::create_dir_all(&root).unwrap();
    fs::write(&cleanup, b"temporary").unwrap();
    let outside = std::env::temp_dir().join(format!(
        "sico-host-outside-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&outside, b"keep").unwrap();
    let mut supervisor = ProcessSupervisor::open(&root).unwrap();
    supervisor.launch(APP, &slow_command()).unwrap();
    supervisor.register_cleanup_file(APP, &cleanup).unwrap();
    assert!(matches!(
        supervisor.register_cleanup_file(APP, &outside),
        Err(LifecycleError::UnsafeCleanup)
    ));
    supervisor.cancel(APP).unwrap();
    assert!(!cleanup.exists());
    assert!(outside.exists());
    fs::remove_file(outside).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
fn slow_command() -> CommandSpec {
    CommandSpec {
        program: OsString::from("cmd.exe"),
        arguments: vec![
            OsString::from("/d"),
            OsString::from("/c"),
            OsString::from("ping -n 6 127.0.0.1 >nul"),
        ],
    }
}

#[cfg(not(windows))]
fn slow_command() -> CommandSpec {
    CommandSpec {
        program: OsString::from("sh"),
        arguments: vec![OsString::from("-c"), OsString::from("sleep 5")],
    }
}

#[cfg(windows)]
fn exit_command(code: i32) -> CommandSpec {
    CommandSpec {
        program: OsString::from("cmd.exe"),
        arguments: vec![
            OsString::from("/d"),
            OsString::from("/c"),
            OsString::from(format!("exit {code}")),
        ],
    }
}

#[cfg(not(windows))]
fn exit_command(code: i32) -> CommandSpec {
    CommandSpec {
        program: OsString::from("sh"),
        arguments: vec![OsString::from("-c"), OsString::from(format!("exit {code}"))],
    }
}

fn test_directory() -> PathBuf {
    std::env::temp_dir().join(format!(
        "sico-lifecycle-test-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ))
}
