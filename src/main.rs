use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::thread;
use std::time::Duration;

use termux_poller::{Config, load_config, network_available, next_backoff, write_example};

const APP: &str = "termux-poller";

fn config_path() -> PathBuf {
    if let Some(path) = env::var_os("TERMUX_POLLER_CONFIG") {
        return path.into();
    }
    PathBuf::from(env::var_os("HOME").unwrap_or_else(|| ".".into()))
        .join(".config")
        .join(APP)
        .join("config.conf")
}

fn usage() {
    eprintln!("Usage: {APP} [--config PATH] <init|once|retry|run>");
}

fn run_command(config: &Config) -> Result<(), String> {
    network_available(config).map_err(|error| format!("network unavailable: {error}"))?;
    let status = Command::new("sh")
        .arg("-c")
        .arg(&config.command)
        .status()
        .map_err(|error| format!("could not start command: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("command exited with {status}"))
}

fn wait(delay: Duration) {
    eprintln!("{APP}: next attempt in {}s", delay.as_secs());
    thread::sleep(delay);
}

// For an external scheduler: return only after a successful run.
fn retry_until_success(config: &Config) {
    let mut delay = None;
    loop {
        if let Some(delay) = delay {
            wait(delay);
        }
        match run_command(config) {
            Ok(()) => return,
            Err(error) => {
                eprintln!("{APP}: {error}; retrying");
                delay = Some(next_backoff(delay, config.retry_min, config.retry_max));
            }
        }
    }
}

// No polling: the process blocks in sleep until it has work to do.
fn run_forever(config: &Config) {
    loop {
        retry_until_success(config);
        eprintln!(
            "{APP}: command completed; next regular run in {}s",
            config.interval.as_secs()
        );
        thread::sleep(config.interval);
    }
}

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let mut path = config_path();
    let action = match args.next() {
        Some(arg) if arg == "--config" => match args.next() {
            Some(value) => {
                path = value.into();
                args.next()
            }
            None => None,
        },
        value => value,
    };
    if args.next().is_some() || action.is_none() {
        usage();
        return ExitCode::from(2);
    }
    let action = action.expect("checked above");
    if action == "init" {
        return match write_example(&path) {
            Ok(()) => {
                println!("Created {}", path.display());
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{APP}: cannot create config: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let config = match load_config(&path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{APP}: cannot load {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    match action.to_string_lossy().as_ref() {
        "once" => match run_command(&config) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{APP}: {error}");
                ExitCode::FAILURE
            }
        },
        "retry" => {
            retry_until_success(&config);
            ExitCode::SUCCESS
        }
        "run" => {
            run_forever(&config);
            ExitCode::SUCCESS
        }
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}
