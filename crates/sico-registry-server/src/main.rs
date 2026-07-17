#![forbid(unsafe_code)]

use std::{
    net::{IpAddr, SocketAddr, TcpListener},
    path::PathBuf,
    process::ExitCode,
    time::Duration,
};

use clap::{Arg, ArgAction, Command, value_parser};
use sico_registry_server::{ServerConfig, serve};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("sico-registry: {message}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let matches = command().get_matches();
    let (name, args) = matches
        .subcommand()
        .ok_or_else(|| "missing command".to_owned())?;
    let root = PathBuf::from(
        args.get_one::<String>("root")
            .expect("root is required by clap"),
    );
    let mut config = ServerConfig::open(&root)
        .map_err(|error| format!("cannot open registry root {}: {error}", root.display()))?;
    if name == "check" {
        config
            .check_ready()
            .map_err(|error| format!("registry root is not ready: {error}"))?;
        println!("REGISTRY_ORIGIN_READY root={}", config.root().display());
        return Ok(());
    }

    config.max_connections = usize::try_from(
        *args
            .get_one::<u64>("max-connections")
            .expect("defaulted by clap"),
    )
    .map_err(|_| "max-connections does not fit this platform".to_owned())?;
    config.max_file_bytes = *args
        .get_one::<u64>("max-file-bytes")
        .expect("defaulted by clap");
    config.max_request_bytes = usize::try_from(
        *args
            .get_one::<u64>("max-request-bytes")
            .expect("defaulted by clap"),
    )
    .map_err(|_| "max-request-bytes does not fit this platform".to_owned())?;
    config.io_timeout = Duration::from_secs(
        *args
            .get_one::<u64>("io-timeout-seconds")
            .expect("defaulted by clap"),
    );
    let address = *args
        .get_one::<SocketAddr>("listen")
        .expect("defaulted by clap");
    if !is_loopback(address.ip()) && !args.get_flag("allow-non-loopback-http") {
        return Err(format!(
            "refusing non-loopback cleartext listener {address}; terminate TLS at an edge and pass --allow-non-loopback-http explicitly"
        ));
    }
    let listener = TcpListener::bind(address)
        .map_err(|error| format!("cannot listen on {address}: {error}"))?;
    let bound = listener
        .local_addr()
        .map_err(|error| format!("cannot read listener address: {error}"))?;
    println!(
        "REGISTRY_ORIGIN_LISTENING address={bound} root={} max_connections={} max_file_bytes={}",
        config.root().display(),
        config.max_connections,
        config.max_file_bytes
    );
    serve(&listener, config).map_err(|error| format!("server stopped: {error}"))
}

fn is_loopback(address: IpAddr) -> bool {
    address.is_loopback()
}

fn command() -> Command {
    Command::new("sico-registry")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Bounded read-only origin for a signed Sico registry tree")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("check")
                .about("Check registry root readiness without listening")
                .arg(root_arg()),
        )
        .subcommand(
            Command::new("serve")
                .about("Serve immutable registry objects over HTTP/1.1")
                .arg(root_arg())
                .arg(
                    Arg::new("listen")
                        .long("listen")
                        .value_name("IP:PORT")
                        .default_value("127.0.0.1:8787")
                        .value_parser(value_parser!(SocketAddr)),
                )
                .arg(
                    Arg::new("allow-non-loopback-http")
                        .long("allow-non-loopback-http")
                        .help(
                            "Acknowledge cleartext exposure outside loopback (normally behind TLS)",
                        )
                        .action(ArgAction::SetTrue),
                )
                .arg(limit_arg("max-connections", "64", "Concurrent connections"))
                .arg(limit_arg(
                    "max-file-bytes",
                    "536870912",
                    "Largest registry object response",
                ))
                .arg(limit_arg(
                    "max-request-bytes",
                    "16384",
                    "Largest accepted request header",
                ))
                .arg(limit_arg(
                    "io-timeout-seconds",
                    "5",
                    "Per-connection read/write timeout",
                )),
        )
}

fn root_arg() -> Arg {
    Arg::new("root")
        .long("root")
        .value_name("DIRECTORY")
        .help("Existing RFC-0024 registry transport root")
        .required(true)
}

fn limit_arg(name: &'static str, default: &'static str, help: &'static str) -> Arg {
    Arg::new(name)
        .long(name)
        .value_name("NUMBER")
        .help(help)
        .default_value(default)
        .value_parser(value_parser!(u64).range(1..))
}
