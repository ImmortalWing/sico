//! Bounded stdio DAP adapter for one identity-bound Sico Program launch.

#![forbid(unsafe_code)]

use std::io::{Read, Write};
use std::path::Path;

use sico_runner::{
    CancelToken, FsGrants, NetGrants, Runner, RunnerLimits, RuntimeDapBackend, RuntimeDapConfig,
    ScriptInput,
};
use sico_tooling_protocol::{
    DapSession, MAX_DAP_FRAME_BYTES, MAX_DAP_HEADER_BYTES, decode_dap_frame, encode_dap_frame,
};

const MAX_COMPONENT_BYTES: usize = 64 * 1024 * 1024;
const MAX_DEBUG_MAP_BYTES: usize = 16 * 1024 * 1024;
const MAX_IDENTITY_BYTES: usize = 1024 * 1024;
const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;

fn main() {
    if let Err(error) = run() {
        eprintln!("sico-dap: {error}");
        std::process::exit(121);
    }
}

fn run() -> Result<(), String> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 5 {
        return Err("usage: sico-dap PROGRAM.component.wasm PROGRAM.debug-map.json PROGRAM.debug-identity.json SOURCE DOCUMENT_ID".to_owned());
    }
    let component = read_bounded(Path::new(&arguments[0]), MAX_COMPONENT_BYTES)?;
    let map = read_bounded(Path::new(&arguments[1]), MAX_DEBUG_MAP_BYTES)?;
    let identity = read_bounded(Path::new(&arguments[2]), MAX_IDENTITY_BYTES)?;
    let source = read_bounded(Path::new(&arguments[3]), MAX_SOURCE_BYTES)?;
    let runner = Runner::new_debug().map_err(|_| "debug-engine-setup-failed".to_owned())?;
    let prepared = runner
        .prepare_program_with_debug(
            &component,
            &map,
            &identity,
            &FsGrants::default(),
            &NetGrants::default(),
        )
        .map_err(|_| "debug-artifact-verification-failed".to_owned())?;
    let backend = RuntimeDapBackend::new(
        prepared,
        RuntimeDapConfig {
            document_id: &arguments[4],
            display_uri: &format!("workspace://{}", arguments[4]),
            source,
            input: ScriptInput::default(),
            limits: RunnerLimits::default(),
            cancel: CancelToken::new(),
            run_id: "dap-stdio-run",
            generation_id: 1,
        },
    )?;
    let mut session = DapSession::new(backend);
    let (frame_sender, frame_receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut input = stdin.lock();
        loop {
            let frame = read_frame(&mut input);
            let finished = !matches!(frame, Ok(Some(_)));
            if frame_sender.send(frame).is_err() || finished {
                break;
            }
        }
    });
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    loop {
        match frame_receiver.recv_timeout(std::time::Duration::from_millis(10)) {
            Ok(Ok(Some(frame))) => {
                let request =
                    decode_dap_frame(&frame).map_err(|_| "invalid-dap-frame".to_owned())?;
                let messages = session
                    .handle(&request)
                    .map_err(|_| "invalid-dap-request".to_owned())?;
                write_messages(&mut output, messages)?;
            }
            Ok(Ok(None)) => break,
            Ok(Err(error)) => return Err(error),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                let messages = session.poll().map_err(|_| "invalid-dap-event".to_owned())?;
                write_messages(&mut output, messages)?;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

fn write_messages(output: &mut impl Write, messages: Vec<serde_json::Value>) -> Result<(), String> {
    if messages.is_empty() {
        return Ok(());
    }
    for message in messages {
        let frame = encode_dap_frame(&message).map_err(|_| "invalid-dap-response".to_owned())?;
        output
            .write_all(&frame)
            .map_err(|_| "dap-output-failed".to_owned())?;
    }
    output.flush().map_err(|_| "dap-output-failed".to_owned())
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let metadata = std::fs::metadata(path).map_err(|_| "input-open-failed".to_owned())?;
    if metadata.len() > limit as u64 {
        return Err("input-limit".to_owned());
    }
    let bytes = std::fs::read(path).map_err(|_| "input-read-failed".to_owned())?;
    if bytes.len() > limit {
        return Err("input-limit".to_owned());
    }
    Ok(bytes)
}

fn read_frame(input: &mut impl Read) -> Result<Option<Vec<u8>>, String> {
    let mut header = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        match input.read(&mut byte) {
            Ok(0) if header.is_empty() => return Ok(None),
            Ok(0) => return Err("truncated-dap-header".to_owned()),
            Ok(_) => header.push(byte[0]),
            Err(_) => return Err("dap-input-failed".to_owned()),
        }
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
        if header.len() > MAX_DAP_HEADER_BYTES + 4 {
            return Err("dap-header-limit".to_owned());
        }
    }
    let header_text = std::str::from_utf8(&header[..header.len() - 4])
        .map_err(|_| "invalid-dap-header".to_owned())?;
    let mut length = None;
    for line in header_text.split("\r\n") {
        let Some((name, value)) = line.split_once(':') else {
            return Err("invalid-dap-header".to_owned());
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            if length.is_some() {
                return Err("duplicate-content-length".to_owned());
            }
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| "invalid-content-length".to_owned())?,
            );
        } else if !name.eq_ignore_ascii_case("Content-Type") {
            return Err("invalid-dap-header".to_owned());
        }
    }
    let length = length.ok_or_else(|| "missing-content-length".to_owned())?;
    if length > MAX_DAP_FRAME_BYTES {
        return Err("dap-frame-limit".to_owned());
    }
    let mut body = vec![0; length];
    input
        .read_exact(&mut body)
        .map_err(|_| "truncated-dap-body".to_owned())?;
    header.extend_from_slice(&body);
    Ok(Some(header))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_reader_accepts_one_exact_frame_and_rejects_oversize() {
        let request = serde_json::json!({"seq": 1, "type": "request", "command": "initialize"});
        let encoded = encode_dap_frame(&request).unwrap();
        let mut input = encoded.as_slice();
        assert_eq!(
            decode_dap_frame(&read_frame(&mut input).unwrap().unwrap()).unwrap(),
            request
        );
        assert!(read_frame(&mut input).unwrap().is_none());

        let oversized = format!("Content-Length: {}\r\n\r\n", MAX_DAP_FRAME_BYTES + 1);
        assert_eq!(
            read_frame(&mut oversized.as_bytes()).unwrap_err(),
            "dap-frame-limit"
        );
    }
}
