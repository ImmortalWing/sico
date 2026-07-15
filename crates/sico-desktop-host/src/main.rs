fn main() {
    std::process::exit(sico_desktop_host::run(
        std::env::args_os(),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    ));
}
