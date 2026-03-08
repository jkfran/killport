use clap::{Arg, Command};
use clap_complete::generate_to;
use clap_complete::shells::{Bash, Fish, Zsh};
use std::env;
use std::fs;

fn build_cli() -> Command {
    Command::new("killport")
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("ports")
                .help("The list of port numbers to kill processes or containers on")
                .required(true)
                .num_args(1..),
        )
        .arg(
            Arg::new("mode")
                .long("mode")
                .short('m')
                .help("Mode of operation: auto (default, kill both), process (only processes), container (only containers)")
                .default_value("auto")
                .value_parser(["auto", "process", "container"]),
        )
        .arg(
            Arg::new("signal")
                .long("signal")
                .short('s')
                .value_name("SIG")
                .help("SIG is a signal name")
                .default_value("sigkill"),
        )
        .arg(
            Arg::new("dry_run")
                .long("dry-run")
                .help("Perform a dry run without killing any processes or containers")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no_fail")
                .long("no-fail")
                .help("Exit successfully even if no matching process or container is found")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Increase logging verbosity")
                .action(clap::ArgAction::Count),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Decrease logging verbosity")
                .action(clap::ArgAction::Count),
        )
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // Generate shell completions
    let completions_dir = format!("{}/completions", out_dir);
    fs::create_dir_all(&completions_dir).unwrap();

    let mut cmd = build_cli();
    generate_to(Bash, &mut cmd, "killport", &completions_dir).unwrap();
    generate_to(Zsh, &mut cmd, "killport", &completions_dir).unwrap();
    generate_to(Fish, &mut cmd, "killport", &completions_dir).unwrap();

    // Generate man page
    let man_dir = format!("{}/man", out_dir);
    fs::create_dir_all(&man_dir).unwrap();

    let cmd = build_cli();
    let man = clap_mangen::Man::new(cmd);
    let mut buffer = Vec::new();
    man.render(&mut buffer).unwrap();
    fs::write(format!("{}/killport.1", man_dir), buffer).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
