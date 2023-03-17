mod port;
mod process;

use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use log::error;
use port::kill_port;
use std::process::exit;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct KillPortArgs {
    #[arg(name = "ports", help = "The list of port numbers to kill processes on")]
    ports: Vec<u16>,

    #[command(flatten)]
    verbose: Verbosity<WarnLevel>,
}

fn main() {
    let args = KillPortArgs::parse();
    let log_level = args
        .verbose
        .log_level()
        .map(|level| level.to_level_filter())
        .unwrap();

    env_logger::Builder::new()
        .format_module_path(log_level == log::LevelFilter::Trace)
        .format_target(log_level == log::LevelFilter::Trace)
        .format_timestamp(Option::None)
        .filter_level(log_level)
        .init();

    for port in args.ports {
        match kill_port(port) {
            Ok(killed) => {
                if killed {
                    println!("Successfully killed processes using port {}.", port);
                } else {
                    println!("No processes found using port {}.", port);
                }
            }
            Err(err) => {
                error!("{}", err);
                exit(1);
            }
        }
    }
}
