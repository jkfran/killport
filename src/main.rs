//! The `killport` command-line utility is designed to kill processes
//! listening on specified ports.
//!
//! The utility accepts a list of port numbers as input and attempts to
//! terminate any processes listening on those ports.

use clap::Parser;
use clap_verbosity_flag::LevelFilter;
use log::error;
use std::io::Write;
use std::process::exit;

use killport::cli::{service_descriptors, KillPortArgs};
use killport::killport::{Killport, KillportOperations};

fn main() {
    // Parse command-line arguments
    let args = KillPortArgs::parse();

    // Set up logging environment
    let log_level = args
        .verbose
        .log_level()
        .map(|level| level.to_level_filter())
        .unwrap();

    env_logger::builder()
        .format(move |buf, record| {
            if log_level <= LevelFilter::Info {
                writeln!(buf, "{}", record.args())
            } else {
                // Default format for lower levels
                writeln!(
                    buf,
                    "[{}] {}: {}",
                    record.target(),
                    record.level(),
                    record.args()
                )
            }
        })
        .format_module_path(log_level == log::LevelFilter::Trace)
        .format_target(log_level == log::LevelFilter::Trace)
        .format_timestamp(Option::None)
        .filter_level(log_level)
        .init();

    let (service_type_singular, _service_type_plural) = service_descriptors(args.mode);

    // Create an instance of Killport
    let killport = Killport;

    // Attempt to kill processes listening on specified ports
    for port in args.ports {
        match killport.kill_service_by_port(port, args.signal.clone(), args.mode, args.dry_run) {
            Ok(killed_services) => {
                if killed_services.is_empty() {
                    println!("No {} found using port {}", service_type_singular, port);
                } else {
                    for (killable_type, name) in killed_services {
                        let action = if args.dry_run {
                            "Would kill"
                        } else {
                            "Successfully killed"
                        };
                        println!(
                            "{} {} '{}' listening on port {}",
                            action, killable_type, name, port
                        );
                    }
                }
            }
            Err(err) => {
                error!("{}", err);
                exit(1);
            }
        }
    }
}
