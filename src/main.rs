use std::net::ToSocketAddrs;

use byte_unit::Byte;
use crab_net::{manager, Parameters};
use log::{info, warn, LevelFilter};
use simple_logger::SimpleLogger;
use clap::{Arg, ArgMatches, Command};
use tokio::runtime::{Builder, Runtime};
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {

    SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();

    let cli = build_cli();
    let rt = build_runtime(&cli);
   
    rt.block_on(async {manager(extract_parameters(cli)).await;});
}
fn build_cli() -> ArgMatches {
    Command::new("UDP TRAFFIC GENERATOR")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
        .about("Simple stress test for servers")
        .arg(
            Arg::new("addr")
                .short('d')
                .long("destination")
                .help("Server address as IP:PORT")
                .required(true)
        )
        .arg(
            Arg::new("clients")
                .short('c')
                .long("connections")
                .help("Number of clients to simulate")
                .default_value("1")
                .value_parser(clap::value_parser!(usize))
        )
        .arg(
            Arg::new("length")
                .short('l')
                .long("length")
                .help("Payload size as bytes")
                .default_value("16")
                .value_parser(clap::value_parser!(usize))
        )
        .arg(
            Arg::new("rate")
                .short('r')
                .long("rate")
                .help("Defined as packets/sec")
                .default_value("1")
                .value_parser(clap::value_parser!(usize))

        ).arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .help("Starting source port for clients")
                .default_value("8000")
                .value_parser(clap::value_parser!(usize))
        ).arg(
            Arg::new("workers")
                .short('w')
                .long("workers")
                .help("Number of worker threads for the Tokio runtime [default: #CPU core]")
                .value_parser(clap::value_parser!(usize))
        ).arg(
            Arg::new("timeout")
                .short('s')
                .long("timeout")
                .help("Timeout between consecutive connections spawn as ms")
                .default_value("50")
                .value_parser(clap::value_parser!(u64))
        ).arg(
            Arg::new("udp")
                .long("udp")
                .help("Send packets via UDP")
                .num_args(0)
                .default_missing_value("true")
                .default_value("false")
                .value_parser(clap::value_parser!(bool))
        ).arg(
            Arg::new("tls")
                .long("tls")
                .help("Send data over TLS")
                .num_args(0)
                .default_missing_value("true")
                .default_value("false")
                .value_parser(clap::value_parser!(bool))
        ).arg(
            Arg::new("ca")
                .long("ca")
                .help("PEM File to validate server credentials")
                .value_parser(clap::value_parser!(String))
        )
        .get_matches()
}

fn build_runtime(cli: &ArgMatches) -> Runtime {

    let worker_threads = cli.get_one::<usize>("workers");
    let mut rt_builder = Builder::new_multi_thread();
    if let Some(workers) = worker_threads {
        if *workers > 0 {
            rt_builder.worker_threads(*workers);
        }

    } else {
        warn!("Workers threads must be > 0. Switching to #CPU Core");
    }

    rt_builder.enable_all()
              .build()
              .unwrap()
}

fn extract_parameters(matches: ArgMatches) -> Parameters {
    let server_addr =  matches.get_one::<String>("addr").unwrap().to_socket_addrs().unwrap().next().unwrap();
    let rate = *matches.get_one("rate").unwrap();
    let connections = *matches.get_one("clients").unwrap();
    let len = *matches.get_one("length").unwrap();
    let start_port = *matches.get_one("port").unwrap();
    let sleep = *matches.get_one("timeout").unwrap();

    let bandwidth = Byte::from_bytes((connections * rate * len * 8) as u128).get_appropriate_unit(false).to_string();
    let bandwidth = bandwidth[0..bandwidth.len()-1].to_string();

    let use_udp = *matches.get_one("udp").unwrap();
    let use_dtls = *matches.get_one("tls").unwrap();
    let ca_file = matches.get_one("ca").cloned();

    info!("Server address: {}, clients: {}, payload size: {}, rate: {} pkt/s, sleep timeout:{} ms, udp: {}, tls: {}",server_addr, connections, len, rate, sleep, use_udp, use_dtls);
    info!("Theoretical Packets rate: {} pkt/sec", connections * rate);
    info!("Theoretical Bandwidth: {}bit/s", bandwidth);

    Parameters::new(server_addr, rate, connections, len, start_port, sleep, (use_udp, (use_dtls, ca_file)))
}