use std::io::{self, BufRead, Write};

use sico_mcp_server::handle_frame;

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(frame) = serde_json::from_str::<serde_json::Value>(&line) else {
            let error = serde_json::json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": "parse error" }
            });
            serde_json::to_writer(&mut output, &error)?;
            output.write_all(b"\n")?;
            output.flush()?;
            continue;
        };
        if let Some(response) = handle_frame(&frame) {
            serde_json::to_writer(&mut output, &response)?;
            output.write_all(b"\n")?;
        }
        output.flush()?;
    }
    Ok(())
}
