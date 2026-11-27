//! hyprhyde: collapse Hyprland windows into round bubbles in a screen corner.

mod daemon;
mod hyprland;
mod icon;
mod parking;

use std::process::ExitCode;

const USAGE: &str = "\
usage: hyprhyde <command>

commands:
  hide                     collapse the active window into a bubble
  daemon [--corner <c>]    show the bubbles; <c> is bottom-right (default),
                           bottom-left, top-right or top-left
  help                     show this text
  version                  print the version";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["hide"] => match parking::hide_active() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => fail(&error.to_string()),
        },
        ["daemon"] => run_daemon(daemon::Corner::default()),
        ["daemon", "--corner", corner] => match corner.parse() {
            Ok(corner) => run_daemon(corner),
            Err(error) => fail(&error),
        },
        ["help" | "--help" | "-h"] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        ["version" | "--version" | "-V"] => {
            println!("hyprhyde {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => fail(USAGE),
    }
}

fn run_daemon(corner: daemon::Corner) -> ExitCode {
    ExitCode::from(daemon::run(corner).get())
}

fn fail(message: &str) -> ExitCode {
    eprintln!("hyprhyde: {message}");
    ExitCode::FAILURE
}
