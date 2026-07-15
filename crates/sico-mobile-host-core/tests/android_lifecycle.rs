use sico_mobile_host_core::{
    AndroidGuestState, AndroidLifecycle, BackendAvailability, LifecycleDescriptor,
    MobileLifecycleError, RuntimeBackend, TerminalReason, engine_config_plan,
    select_runtime_backend,
};

const APP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const REV: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn backend_selection_prefers_proven_native_then_pulley() {
    assert_eq!(
        select_runtime_backend(BackendAvailability {
            native_compiled: true,
            executable_memory: true,
            pulley_compiled: true,
        }),
        Ok(RuntimeBackend::NativeCranelift)
    );
    assert_eq!(
        select_runtime_backend(BackendAvailability {
            native_compiled: true,
            executable_memory: false,
            pulley_compiled: true,
        }),
        Ok(RuntimeBackend::Pulley)
    );
    assert_eq!(
        engine_config_plan(RuntimeBackend::NativeCranelift).target,
        "android-native"
    );
    assert_eq!(engine_config_plan(RuntimeBackend::Pulley).target, "pulley64");
}

#[test]
fn foreground_background_and_restore_require_reverified_descriptor() {
    let descriptor = descriptor();
    let mut lifecycle = AndroidLifecycle::create(descriptor.clone()).unwrap();
    lifecycle.foreground().unwrap();
    lifecycle.background().unwrap();
    assert_eq!(lifecycle.state(), AndroidGuestState::BackgroundSuspended);
    let restored = AndroidLifecycle::restore_after_process_death(descriptor).unwrap();
    assert_eq!(restored.state(), AndroidGuestState::Created);
}

#[test]
fn duplicate_open_queue_is_bounded_and_ordered() {
    let mut lifecycle = AndroidLifecycle::create(descriptor()).unwrap();
    for index in 0..256 {
        lifecycle.queue_open(format!("{index:064x}")).unwrap();
    }
    assert_eq!(
        lifecycle.queue_open("c".repeat(64)),
        Err(MobileLifecycleError::OpenQueueFull)
    );
    assert_eq!(lifecycle.take_open().unwrap(), format!("{:064x}", 0));
}

#[test]
fn terminal_races_have_one_winner_and_clear_events() {
    let mut lifecycle = AndroidLifecycle::create(descriptor()).unwrap();
    lifecycle.queue_open("c".repeat(64)).unwrap();
    lifecycle.finish(TerminalReason::TimedOut).unwrap();
    assert_eq!(
        lifecycle.state(),
        AndroidGuestState::Terminal(TerminalReason::TimedOut)
    );
    assert_eq!(
        lifecycle.finish(TerminalReason::Cancelled),
        Err(MobileLifecycleError::Terminal)
    );
    assert!(lifecycle.take_open().is_none());
}

#[test]
fn malformed_descriptor_and_unavailable_runtime_fail_closed() {
    assert!(
        AndroidLifecycle::create(LifecycleDescriptor {
            app_identity: "app".to_owned(),
            revision_digest: REV.to_owned(),
        })
        .is_err()
    );
    assert_eq!(
        select_runtime_backend(BackendAvailability {
            native_compiled: false,
            executable_memory: false,
            pulley_compiled: false,
        }),
        Err(MobileLifecycleError::RuntimeUnavailable)
    );
}

fn descriptor() -> LifecycleDescriptor {
    LifecycleDescriptor {
        app_identity: APP.to_owned(),
        revision_digest: REV.to_owned(),
    }
}
