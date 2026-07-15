use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::windows_association_plan;

pub const SAPP_EXTENSION: &str = ".sapp";
pub const SAPP_MEDIA_TYPE: &str = "application/vnd.sico.sapp";
pub const SAPP_UTI: &str = "dev.sico.sapp";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlatformKind {
    Windows,
    Macos,
    Linux,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceLevel {
    RuntimeVerified,
    ContractVerified,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlatformContract {
    pub schema: String,
    pub platform: PlatformKind,
    pub evidence: EvidenceLevel,
    pub extension: String,
    pub media_type: String,
    pub content_identity: String,
    pub open_delivery: String,
    pub invokes_shell: bool,
    pub trust_owner: String,
}

/// Returns the frozen parity contract for all three desktop adapters.
#[must_use]
pub fn contracts() -> Vec<PlatformContract> {
    [
        (PlatformKind::Windows, EvidenceLevel::RuntimeVerified),
        (PlatformKind::Macos, EvidenceLevel::ContractVerified),
        (PlatformKind::Linux, EvidenceLevel::ContractVerified),
    ]
    .into_iter()
    .map(|(platform, evidence)| PlatformContract {
        schema: "sico.desktop.platform-contract.v0".to_owned(),
        platform,
        evidence,
        extension: SAPP_EXTENSION.to_owned(),
        media_type: SAPP_MEDIA_TYPE.to_owned(),
        content_identity: "package-digest+app-signer".to_owned(),
        open_delivery: "single-path-argument".to_owned(),
        invokes_shell: false,
        trust_owner: "sico-host-core".to_owned(),
    })
    .collect()
}

/// Writes inspectable Windows, macOS and Linux association artifacts.
///
/// # Errors
///
/// Rejects invalid Windows executable paths, existing output directories and
/// all serialization or I/O failures.
pub fn write_artifacts(
    output: &Path,
    windows_executable: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if output.exists() {
        return Err("platform artifact output already exists".into());
    }
    fs::create_dir_all(output.join("macos"))?;
    fs::create_dir_all(output.join("linux"))?;
    fs::write(
        output.join("contracts.json"),
        serde_json::to_vec_pretty(&contracts())?,
    )?;
    fs::write(
        output.join("windows-association-plan.json"),
        serde_json::to_vec_pretty(&windows_association_plan(windows_executable)?)?,
    )?;
    fs::write(output.join("macos/Info.plist"), macos_info_plist())?;
    fs::write(output.join("linux/sico.desktop"), linux_desktop_entry())?;
    fs::write(output.join("linux/sico-sapp.xml"), linux_mime_package())?;
    fs::write(output.join("linux/mimeapps.list"), linux_mimeapps_list())?;
    Ok(())
}

#[must_use]
pub fn macos_info_plist() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleDocumentTypes</key><array><dict>
<key>CFBundleTypeName</key><string>Sico Application Package</string>
<key>CFBundleTypeRole</key><string>Viewer</string>
<key>LSHandlerRank</key><string>Alternate</string>
<key>LSItemContentTypes</key><array><string>dev.sico.sapp</string></array>
</dict></array>
<key>UTExportedTypeDeclarations</key><array><dict>
<key>UTTypeIdentifier</key><string>dev.sico.sapp</string>
<key>UTTypeDescription</key><string>Sico Application Package</string>
<key>UTTypeConformsTo</key><array><string>public.data</string></array>
<key>UTTypeTagSpecification</key><dict>
<key>public.filename-extension</key><array><string>sapp</string></array>
<key>public.mime-type</key><string>application/vnd.sico.sapp</string>
</dict></dict></array>
</dict></plist>
"#
}

#[must_use]
pub fn linux_desktop_entry() -> &'static str {
    "[Desktop Entry]\nType=Application\nName=Sico Desktop Host\nExec=sico-desktop-host open %f\nIcon=sico-desktop-host\nTerminal=false\nNoDisplay=true\nMimeType=application/vnd.sico.sapp;\n"
}

#[must_use]
pub fn linux_mime_package() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/vnd.sico.sapp">
    <comment>Sico Application Package</comment>
    <glob pattern="*.sapp"/>
  </mime-type>
</mime-info>
"#
}

#[must_use]
pub fn linux_mimeapps_list() -> &'static str {
    "[Added Associations]\napplication/vnd.sico.sapp=sico.desktop;\n"
}
