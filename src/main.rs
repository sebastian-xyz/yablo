extern crate battery;
extern crate clap;
extern crate num_cpus;
extern crate systemstat;
extern crate toml;

use clap::{App, Arg};
use systemstat::{Platform, System};
use crossterm::ExecutableCommand;
use crossterm::style::Colorize;

mod lib;

fn main() {
    let matches = App::new("yablo")
        .version("0.1.0")
        .about("Yet Another Battery Life Optimizer for Linux")
        .long_about("Yet Another Battery Life Optimizer for Linux (yablo) automatically sets cpu governor and turbo boost to save energy.")
        .arg(
            Arg::with_name("daemon")
                .long("daemon")
                .hidden(true)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("debug")
                .short("d")
                .long("debug")
                .help("Shows debug/system info")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("live")
                .short("l")
                .long("live")
                .help("Prints information and applies suggested CPU optimizations")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("log")
                .long("log")
                .help("View live CPU optimization log made by daemon")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("monitor")
                .short("m")
                .long("monitor")
                .help("Suggests CPU optimizations for the current load")
                .takes_value(false),
        )
        .get_matches();

    if matches.is_present("daemon") {
        lib::check_root();
        let mut stdout = std::io::stdout();
        match stdout.execute(crossterm::terminal::EnterAlternateScreen) {
            Ok(_) => (),
            Err(x) => {
                eprintln!("[{}] Error: {}", "!".red(), x);
                std::process::exit(1)
            }
        };
        lib::check_config_existence();
        let config = lib::get_config();
        lib::check_config_errors(&config);
        let num_cores = num_cpus::get() as i32;
        let sys = System::new();
        let (turbo_available, invert_turbo) = lib::check_turbo_availability();
        lib::check_daemon();
        lib::check_log();
        let mut daemon_count = 0;
        loop {
            let sys_info = lib::get_sys_info(&sys, turbo_available, invert_turbo, num_cores);
            lib::print_info(&sys_info, &mut stdout);
            lib::optimize_powerstate(
                &config,
                &sys_info,
                num_cores,
                &mut daemon_count,
                &mut stdout,
            );
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    } else if matches.is_present("monitor") {
        let mut stdout = std::io::stdout();
        match stdout.execute(crossterm::terminal::EnterAlternateScreen) {
            Ok(_) => (),
            Err(x) => {
                eprintln!("[{}] Error: {}", "!".red(),  x);
                std::process::exit(1)
            }
        };
        lib::check_config_existence();
        let config = lib::get_config();
        lib::check_config_errors(&config);
        let num_cores = num_cpus::get() as i32;
        let sys = System::new();
        let (turbo_available, invert_turbo) = lib::check_turbo_availability();
        let mut monitor_count = 0;
        loop {
            let sys_info = lib::get_sys_info(&sys, turbo_available, invert_turbo, num_cores);
            lib::print_info(&sys_info, &mut stdout);
            lib::monitor_state(
                &config,
                &sys_info,
                num_cores,
                &mut monitor_count,
                &mut stdout,
            );
            match lib::quit_program(3000, &mut stdout) {
                Ok(_) => (),
                Err(err) => {
                    eprintln!("Error: {}", err);
                    std::process::exit(1)
                }
            };
        }
    } else if matches.is_present("live") {
        lib::check_root();
        let mut stdout = std::io::stdout();
        match stdout.execute(crossterm::terminal::EnterAlternateScreen) {
            Ok(_) => (),
            Err(x) => {
                eprintln!("[{}] Error: {}", "!".red(), x);
                std::process::exit(1)
            }
        };
        lib::check_config_existence();
        let config = lib::get_config();
        lib::check_config_errors(&config);
        let num_cores = num_cpus::get() as i32;
        let sys = System::new();
        let (turbo_available, invert_turbo) = lib::check_turbo_availability();
        let mut live_count = 0;
        lib::check_daemon();
        loop {
            let sys_info = lib::get_sys_info(&sys, turbo_available, invert_turbo, num_cores);
            lib::print_info(&sys_info, &mut stdout);
            lib::optimize_powerstate(&config, &sys_info, num_cores, &mut live_count, &mut stdout);
            println!("{}", ":".repeat(50));
            match lib::quit_program(3000, &mut stdout) {
                Ok(_) => (),
                Err(err) => {
                    eprintln!("Error: {}", err);
                    std::process::exit(1)
                }
            };
        }
    } else if matches.is_present("log") {
        let num_cores = num_cpus::get() as i32;
        let mut stdout = std::io::stdout();
        match stdout.execute(crossterm::terminal::EnterAlternateScreen) {
            Ok(_) => (),
            Err(x) => {
                eprintln!("[{}] Error: {}", "!".red(), x);
                std::process::exit(1)
            }
        };
        loop {
            lib::print_log(num_cores, &mut stdout);
            match lib::quit_program(500, &mut stdout) {
                Ok(_) => (),
                Err(err) => {
                    eprintln!("Error: {}", err);
                    std::process::exit(1)
                }
            };
        }
    } else if matches.is_present("debug") {
        let mut stdout = std::io::stdout();
        match stdout.execute(crossterm::terminal::EnterAlternateScreen) {
            Ok(_) => (),
            Err(x) => {
                eprintln!("[{}] Error: {}", "!".red(), x);
                std::process::exit(1)
            }
        };
        let sys = System::new();
        let (turbo_available, invert_turbo) = lib::check_turbo_availability();
        let num_cores = num_cpus::get() as i32;
        loop {
            let sys_info = lib::get_sys_info(&sys, turbo_available, invert_turbo, num_cores);
            lib::print_info(&sys_info, &mut stdout)
            match lib::quit_program(500, &mut stdout) {
                Ok(_) => (),
                Err(err) => {
                    eprintln!("Error: {}", err);
                    std::process::exit(1)
                }
            };
        }
    } else if matches.is_present("update") {
        lib::check_root();
        lib::restart_daemon();
        println!(
            "[{}] Successfully restarted daemon. New config loaded.",
            "+".green()
        );
    } else {
        println!("Type 'yablo --help' to get available options");
    }
}
