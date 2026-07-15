use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

use sico_desktop_host::{
    EvidenceLevel, PlatformKind, linux_desktop_entry, linux_mime_package, macos_info_plist,
    platform_contracts, write_platform_artifacts,
};

static TEST_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn every_adapter_preserves_identity_open_and_trust_ownership() {
    let contracts = platform_contracts();
    assert_eq!(contracts.len(), 3);
    for contract in &contracts {
        assert_eq!(contract.extension, ".sapp");
        assert_eq!(contract.media_type, "application/vnd.sico.sapp");
        assert_eq!(contract.content_identity, "package-digest+app-signer");
        assert_eq!(contract.open_delivery, "single-path-argument");
        assert!(!contract.invokes_shell);
        assert_eq!(contract.trust_owner, "sico-host-core");
    }
    assert_eq!(contracts[0].platform, PlatformKind::Windows);
    assert_eq!(contracts[0].evidence, EvidenceLevel::RuntimeVerified);
    assert!(
        contracts[1..]
            .iter()
            .all(|contract| contract.evidence == EvidenceLevel::ContractVerified)
    );
}

#[test]
fn macos_and_linux_declarations_are_open_with_not_silent_default() {
    let plist = macos_info_plist();
    assert!(plist.contains("CFBundleDocumentTypes"));
    assert!(plist.contains("dev.sico.sapp"));
    assert!(plist.contains("LSHandlerRank</key><string>Alternate"));
    assert!(plist.contains("application/vnd.sico.sapp"));

    let desktop = linux_desktop_entry();
    assert!(desktop.contains("Exec=sico-desktop-host open %f"));
    assert!(desktop.contains("NoDisplay=true"));
    assert!(!desktop.contains("sh -c"));
    assert!(linux_mime_package().contains("<glob pattern=\"*.sapp\"/>"));
}

#[test]
fn artifact_writer_emits_the_complete_auditable_set() {
    let output = std::env::temp_dir().join(format!(
        "sico-platform-artifacts-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    write_platform_artifacts(&output, &std::env::current_exe().unwrap()).unwrap();
    for relative in [
        "contracts.json",
        "windows-association-plan.json",
        "macos/Info.plist",
        "linux/sico.desktop",
        "linux/sico-sapp.xml",
        "linux/mimeapps.list",
    ] {
        assert!(output.join(relative).is_file(), "missing {relative}");
    }
    assert!(write_platform_artifacts(&output, &std::env::current_exe().unwrap()).is_err());
    fs::remove_dir_all(output).unwrap();
}
