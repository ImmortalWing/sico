use std::io::{self, BufReader, BufWriter, Write as _};

use sico_language_server::{LanguageServer, read_message, write_message};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());
    let mut server = LanguageServer::default();

    while let Some(message) = read_message(&mut reader)? {
        for response in server.process(message) {
            write_message(&mut writer, &response)?;
        }
        writer.flush()?;
        if server.has_exited() {
            break;
        }
    }
    Ok(())
}
