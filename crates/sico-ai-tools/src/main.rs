use std::io::{self, Read as _, Write as _};

use sico_ai_tools::{MAX_REQUEST_BYTES, execute_bytes};

fn main() -> io::Result<()> {
    let mut input = Vec::new();
    io::stdin()
        .take(u64::try_from(MAX_REQUEST_BYTES + 1).expect("request limit fits u64"))
        .read_to_end(&mut input)?;
    let output = execute_bytes(&input);
    serde_json::to_writer(io::stdout().lock(), &output)?;
    io::stdout().lock().write_all(b"\n")
}
