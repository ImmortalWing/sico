//! Windows-first native Desktop Host entrypoint.

#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use clap::{Arg, ArgAction, ArgMatches, Command as ClapCommand};
use serde::Serialize;
use sico_host_core::{
    HostStore, InstallOptions, PermissionChoice, PermissionOutcome, PermissionSession,
    PermissionStore, RenderPlan, UiModel,
};
use sico_runtime::{HostLimits, prepare_storage, run_authorized_package};

mod platform;
pub use platform::{
    EvidenceLevel, PlatformContract, PlatformKind, SAPP_EXTENSION, SAPP_MEDIA_TYPE, SAPP_UTI,
    contracts as platform_contracts, linux_desktop_entry, linux_mime_package, linux_mimeapps_list,
    macos_info_plist, write_artifacts as write_platform_artifacts,
};

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_ERROR: i32 = 2;

pub fn run<I, T>(args: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = match command().try_get_matches_from(args) {
        Ok(matches) => matches,
        Err(error) => {
            let _ = write!(stderr, "{error}");
            return error.exit_code();
        }
    };
    match matches.subcommand() {
        Some(("install", matches)) => install_command(matches, stdout, stderr),
        Some(("open", matches)) => open_command(matches, stdout, stderr),
        Some(("uninstall", matches)) => uninstall_command(matches, stdout, stderr),
        Some(("association-plan", matches)) => association_plan_command(matches, stdout, stderr),
        Some(("association-apply", matches)) => association_apply_command(matches, stderr),
        Some(("association-remove", _)) => association_remove_command(stderr),
        Some(("platform-artifacts", matches)) => {
            platform_artifacts_command(matches, stdout, stderr)
        }
        Some(("ui-preview", matches)) => ui_preview_command(matches, stdout, stderr),
        _ => EXIT_ERROR,
    }
}

fn command() -> ClapCommand {
    ClapCommand::new("sico-desktop-host")
        .about("Install and open verified Sico .sapp applications")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            ClapCommand::new("install")
                .arg(Arg::new("package").required(true))
                .arg(store_arg())
                .arg(trusted_key_arg())
                .arg(
                    Arg::new("allow-downgrade")
                        .long("allow-downgrade")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(open_subcommand())
        .subcommand(
            ClapCommand::new("uninstall")
                .arg(Arg::new("app-identity").required(true))
                .arg(store_arg()),
        )
        .subcommand(
            ClapCommand::new("association-plan")
                .arg(Arg::new("executable").long("executable").required(true)),
        )
        .subcommand(
            ClapCommand::new("association-apply")
                .arg(Arg::new("executable").long("executable").required(true)),
        )
        .subcommand(ClapCommand::new("association-remove"))
        .subcommand(
            ClapCommand::new("platform-artifacts")
                .arg(Arg::new("output").long("output").required(true))
                .arg(Arg::new("executable").long("executable").required(true)),
        )
        .subcommand(
            ClapCommand::new("ui-preview")
                .arg(Arg::new("model").required(true))
                .arg(
                    Arg::new("validate-only")
                        .long("validate-only")
                        .action(ArgAction::SetTrue),
                ),
        )
}

fn open_subcommand() -> ClapCommand {
    ClapCommand::new("open")
        .arg(Arg::new("package").required(true))
        .arg(store_arg())
        .arg(trusted_key_arg())
        .arg(
            Arg::new("runtime")
                .long("runtime")
                .required(true)
                .value_name("WASMTIME"),
        )
        .arg(
            Arg::new("grant")
                .long("grant")
                .action(ArgAction::Append)
                .value_name("CAPABILITY"),
        )
        .arg(Arg::new("permission").long("permission").value_parser([
            "deny",
            "allow-once",
            "allow-persistent",
        ]))
}

fn store_arg() -> Arg {
    Arg::new("store")
        .long("store")
        .required(true)
        .value_name("DIRECTORY")
}

fn trusted_key_arg() -> Arg {
    Arg::new("trusted-key")
        .long("trusted-key")
        .required(true)
        .value_name("PUBLIC_KEY_FILE")
}

fn install_command(matches: &ArgMatches, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let result = (|| -> Result<_, Box<dyn std::error::Error>> {
        let store = HostStore::open(path_arg(matches, "store"))?;
        let keys = BTreeSet::from([read_key(path_arg(matches, "trusted-key"))?]);
        Ok(store.install_path(
            path_arg(matches, "package"),
            &keys,
            InstallOptions {
                allow_downgrade: matches.get_flag("allow-downgrade"),
            },
        )?)
    })();
    match result {
        Ok(installed) => {
            let output = serde_json::to_string(&installed).expect("installed metadata serializes");
            if writeln!(stdout, "{output}").is_ok() {
                EXIT_SUCCESS
            } else {
                EXIT_ERROR
            }
        }
        Err(error) => fail(stderr, &error),
    }
}

fn open_command(matches: &ArgMatches, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match prepare_open(matches) {
        Ok((opened, storage, runtime)) => {
            let output = run_authorized_package(
                &runtime,
                &opened.package,
                storage.as_ref(),
                &HostLimits::default(),
            );
            match output {
                Ok(output) => {
                    if stdout.write_all(&output.stdout).is_err()
                        || stderr.write_all(&output.stderr).is_err()
                    {
                        return EXIT_ERROR;
                    }
                    if output.status.success() {
                        EXIT_SUCCESS
                    } else {
                        let _ = writeln!(stderr, "Desktop guest terminated: {:?}", output.fault);
                        EXIT_ERROR
                    }
                }
                Err(error) => fail(stderr, &error),
            }
        }
        Err(error) => fail(stderr, &error),
    }
}

type PreparedOpen = (
    sico_host_core::OpenedPackage,
    Option<sico_runtime::AppStorage>,
    OsString,
);

fn prepare_open(matches: &ArgMatches) -> Result<PreparedOpen, Box<dyn std::error::Error>> {
    let store = HostStore::open(path_arg(matches, "store"))?;
    let keys = BTreeSet::from([read_key(path_arg(matches, "trusted-key"))?]);
    let installed = store.install_path(
        path_arg(matches, "package"),
        &keys,
        InstallOptions::default(),
    )?;
    let grants: BTreeSet<String> = matches
        .get_many::<String>("grant")
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    let opened = store.open_installed(
        &installed.app_identity,
        &installed.revision_digest,
        &keys,
        &grants,
    )?;
    apply_permissions(matches, &store, &opened)?;
    let limits = HostLimits::default();
    let storage = if opened
        .package
        .granted_capabilities
        .contains("storage.read-write")
    {
        Some(prepare_storage(
            &store.root().join("storage"),
            &opened.package,
            limits.storage_bytes,
        )?)
    } else {
        None
    };
    Ok((
        opened,
        storage,
        matches.get_one::<String>("runtime").unwrap().into(),
    ))
}

fn apply_permissions(
    matches: &ArgMatches,
    store: &HostStore,
    opened: &sico_host_core::OpenedPackage,
) -> Result<(), Box<dyn std::error::Error>> {
    let permissions = PermissionStore::open(store.root())?;
    let mut session = PermissionSession::default();
    if matches!(
        permissions.resolve(opened, &session)?,
        PermissionOutcome::Granted(_)
    ) {
        return Ok(());
    }
    let prompt = PermissionStore::prompt(opened);
    let choice = matches
        .get_one::<String>("permission")
        .map(|choice| parse_choice(choice))
        .transpose()?
        .unwrap_or(native_permission_dialog(&prompt)?);
    let decisions: BTreeMap<_, _> = prompt
        .capabilities
        .iter()
        .map(|capability| (capability.clone(), choice))
        .collect();
    match permissions.apply(opened, &decisions, &mut session)? {
        PermissionOutcome::Granted(_) => Ok(()),
        PermissionOutcome::Denied(_) | PermissionOutcome::PromptRequired(_) => {
            Err("permission denied".into())
        }
    }
}

fn parse_choice(choice: &str) -> Result<PermissionChoice, Box<dyn std::error::Error>> {
    match choice {
        "deny" => Ok(PermissionChoice::Deny),
        "allow-once" => Ok(PermissionChoice::AllowOnce),
        "allow-persistent" => Ok(PermissionChoice::AllowPersistent),
        _ => Err("unknown permission choice".into()),
    }
}

fn uninstall_command(matches: &ArgMatches, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let result = (|| -> Result<_, Box<dyn std::error::Error>> {
        let store = HostStore::open(path_arg(matches, "store"))?;
        let app_identity = string_arg(matches, "app-identity");
        let removed = store.uninstall(app_identity)?;
        PermissionStore::open(store.root())?.remove_app(app_identity)?;
        Ok(removed)
    })();
    match result {
        Ok(removed) => {
            if writeln!(stdout, "removed={removed}").is_ok() {
                EXIT_SUCCESS
            } else {
                EXIT_ERROR
            }
        }
        Err(error) => fail(stderr, &error),
    }
}

fn association_plan_command(
    matches: &ArgMatches,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match windows_association_plan(path_arg(matches, "executable")) {
        Ok(plan) => {
            let output = serde_json::to_string_pretty(&plan).expect("association plan serializes");
            if writeln!(stdout, "{output}").is_ok() {
                EXIT_SUCCESS
            } else {
                EXIT_ERROR
            }
        }
        Err(error) => fail(stderr, &error),
    }
}

fn association_apply_command(matches: &ArgMatches, stderr: &mut dyn Write) -> i32 {
    match windows_association_plan(path_arg(matches, "executable"))
        .and_then(|plan| apply_windows_association(&plan))
    {
        Ok(()) => EXIT_SUCCESS,
        Err(error) => fail(stderr, &error),
    }
}

fn association_remove_command(stderr: &mut dyn Write) -> i32 {
    match remove_windows_association() {
        Ok(()) => EXIT_SUCCESS,
        Err(error) => fail(stderr, &error),
    }
}

fn platform_artifacts_command(
    matches: &ArgMatches,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    match write_platform_artifacts(path_arg(matches, "output"), path_arg(matches, "executable")) {
        Ok(()) => {
            if writeln!(stdout, "platform-artifacts=windows,macos,linux").is_ok() {
                EXIT_SUCCESS
            } else {
                EXIT_ERROR
            }
        }
        Err(error) => fail(stderr, &error),
    }
}

fn ui_preview_command(matches: &ArgMatches, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let result = (|| {
        let bytes = fs::read(path_arg(matches, "model"))?;
        let model: UiModel = serde_json::from_slice(&bytes)?;
        let plan = model.validate()?;
        if !matches.get_flag("validate-only") {
            show_native_ui_preview(&plan)?;
        }
        Ok::<_, Box<dyn std::error::Error>>(plan.nodes.len())
    })();
    match result {
        Ok(nodes) => {
            if writeln!(stdout, "validated-ui-nodes={nodes}").is_ok() {
                EXIT_SUCCESS
            } else {
                EXIT_ERROR
            }
        }
        Err(error) => fail(stderr, &error),
    }
}

fn path_arg<'a>(matches: &'a ArgMatches, name: &str) -> &'a Path {
    Path::new(string_arg(matches, name))
}

fn string_arg<'a>(matches: &'a ArgMatches, name: &str) -> &'a str {
    matches.get_one::<String>(name).unwrap()
}

fn read_key(path: &Path) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let text = text.trim();
    if text.len() != 64
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("trusted key must contain 64 lowercase hex characters".into());
    }
    let mut key = [0_u8; 32];
    for (index, output) in key.iter_mut().enumerate() {
        *output = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)?;
    }
    Ok(key)
}

fn fail(stderr: &mut dyn Write, error: &dyn std::fmt::Display) -> i32 {
    let _ = writeln!(stderr, "sico-desktop-host: {error}");
    EXIT_ERROR
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RegistryOperation {
    pub key: String,
    pub name: String,
    pub value: String,
}

/// Creates the exact per-user Windows registry write plan for `.sapp`.
///
/// # Errors
///
/// Rejects missing, non-canonical or non-`.exe` executable paths.
pub fn windows_association_plan(
    executable: &Path,
) -> Result<Vec<RegistryOperation>, Box<dyn std::error::Error>> {
    let executable = executable.canonicalize()?;
    if executable.extension() != Some(OsStr::new("exe")) {
        return Err("association executable must be an .exe".into());
    }
    let quoted = format!("\"{}\" open \"%1\"", executable.display());
    let icon = executable.with_file_name("sico-desktop-host.ico");
    Ok(vec![
        operation(
            r"HKCU\Software\Classes\.sapp\OpenWithProgids",
            "Sico.Sapp",
            "",
        ),
        operation(
            r"HKCU\Software\Classes\Sico.Sapp",
            "",
            "Sico Application Package",
        ),
        operation(
            r"HKCU\Software\Classes\Sico.Sapp\DefaultIcon",
            "",
            &icon.display().to_string(),
        ),
        operation(
            r"HKCU\Software\Classes\Sico.Sapp\shell\open\command",
            "",
            &quoted,
        ),
        operation(
            r"HKCU\Software\Classes\Applications\sico-desktop-host.exe\SupportedTypes",
            ".sapp",
            "",
        ),
    ])
}

fn operation(key: &str, name: &str, value: &str) -> RegistryOperation {
    RegistryOperation {
        key: key.to_owned(),
        name: name.to_owned(),
        value: value.to_owned(),
    }
}

#[cfg(windows)]
fn apply_windows_association(
    operations: &[RegistryOperation],
) -> Result<(), Box<dyn std::error::Error>> {
    for operation in operations {
        let mut command = Command::new("reg.exe");
        command.args(["add", &operation.key, "/f"]);
        if operation.name.is_empty() {
            command.arg("/ve");
        } else {
            command.args(["/v", &operation.name]);
        }
        let status = command
            .args(["/d", &operation.value])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !status.success() {
            return Err("Windows association registration failed".into());
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn apply_windows_association(
    _operations: &[RegistryOperation],
) -> Result<(), Box<dyn std::error::Error>> {
    Err("Windows association adapter is unavailable".into())
}

#[cfg(windows)]
fn remove_windows_association() -> Result<(), Box<dyn std::error::Error>> {
    for key in [
        r"HKCU\Software\Classes\Sico.Sapp",
        r"HKCU\Software\Classes\Applications\sico-desktop-host.exe",
    ] {
        let _ = Command::new("reg.exe")
            .args(["delete", key, "/f"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn remove_windows_association() -> Result<(), Box<dyn std::error::Error>> {
    Err("Windows association adapter is unavailable".into())
}

#[cfg(windows)]
fn native_permission_dialog(
    prompt: &sico_host_core::PermissionPrompt,
) -> Result<PermissionChoice, Box<dyn std::error::Error>> {
    if let Ok(choice) = std::env::var("SICO_TEST_PERMISSION_CHOICE") {
        return parse_choice(&choice);
    }
    let text = format!(
        "Application: {} {}\r\nSigner: {}\r\nPackage: {}\r\n\r\nCapabilities:\r\n{}",
        prompt.app_id,
        prompt.app_version,
        prompt.signer_fingerprint,
        prompt.package_digest,
        prompt.capabilities.join("\r\n")
    );
    let status = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-STA",
            "-Command",
            PERMISSION_DIALOG_SCRIPT,
        ])
        .env("SICO_PERMISSION_TEXT", text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    match status.code() {
        Some(11) => Ok(PermissionChoice::AllowOnce),
        Some(12) => Ok(PermissionChoice::AllowPersistent),
        Some(13) => Ok(PermissionChoice::Deny),
        _ => Err("native permission dialog failed".into()),
    }
}

#[cfg(not(windows))]
fn native_permission_dialog(
    _prompt: &sico_host_core::PermissionPrompt,
) -> Result<PermissionChoice, Box<dyn std::error::Error>> {
    Err("native permission dialog is unavailable".into())
}

#[cfg(windows)]
const PERMISSION_DIALOG_SCRIPT: &str = r"
Add-Type -AssemblyName System.Windows.Forms
$form = New-Object System.Windows.Forms.Form
$form.Text = 'Sico permission request'
$form.Width = 720; $form.Height = 480; $form.StartPosition = 'CenterScreen'
$text = New-Object System.Windows.Forms.TextBox
$text.Multiline = $true; $text.ReadOnly = $true; $text.ScrollBars = 'Vertical'
$text.SetBounds(20,20,660,330); $text.Text = $env:SICO_PERMISSION_TEXT
$once = New-Object System.Windows.Forms.Button; $once.Text = 'Allow once'; $once.SetBounds(230,370,130,36)
$always = New-Object System.Windows.Forms.Button; $always.Text = 'Always allow'; $always.SetBounds(370,370,130,36)
$deny = New-Object System.Windows.Forms.Button; $deny.Text = 'Deny'; $deny.SetBounds(510,370,130,36)
$once.Add_Click({$form.Tag=11;$form.Close()}); $always.Add_Click({$form.Tag=12;$form.Close()}); $deny.Add_Click({$form.Tag=13;$form.Close()})
$form.Controls.AddRange(@($text,$once,$always,$deny)); [void]$form.ShowDialog()
if ($null -eq $form.Tag) { exit 13 } else { exit $form.Tag }
";

#[cfg(windows)]
fn show_native_ui_preview(plan: &RenderPlan) -> Result<(), Box<dyn std::error::Error>> {
    let text = plan
        .nodes
        .iter()
        .filter_map(|node| node.escaped_text.as_deref())
        .collect::<Vec<_>>()
        .join("\r\n");
    let status = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-STA",
            "-Command",
            UI_PREVIEW_SCRIPT,
        ])
        .env("SICO_UI_TEXT", text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err("native UI preview failed".into())
    }
}

#[cfg(not(windows))]
fn show_native_ui_preview(_plan: &RenderPlan) -> Result<(), Box<dyn std::error::Error>> {
    Err("native UI preview is unavailable".into())
}

#[cfg(windows)]
const UI_PREVIEW_SCRIPT: &str = r"
Add-Type -AssemblyName System.Windows.Forms
$form = New-Object System.Windows.Forms.Form; $form.Text = 'Sico App'; $form.Width = 640; $form.Height = 420
$label = New-Object System.Windows.Forms.Label; $label.SetBounds(24,24,570,320); $label.Text = $env:SICO_UI_TEXT
$form.Controls.Add($label); [void]$form.ShowDialog(); exit 0
";
