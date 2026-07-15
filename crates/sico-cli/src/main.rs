#![forbid(unsafe_code)]

fn main() {
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let code = sico_cli::run(std::env::args_os(), &mut stdin, &mut stdout, &mut stderr);
    std::process::exit(code);
}
