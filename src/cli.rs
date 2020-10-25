use clap::{App, Arg};

pub fn build_cli() -> App<'static, 'static> {
    App::new("yablo")
        .version("0.2.0")
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
                .help("View live CPU optimization made by daemon")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("monitor")
                .short("m")
                .long("monitor")
                .help("Suggests CPU optimizations for the current load")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("update")
                .short("u")
                .long("update-config")
                .help("Reloads the systemd daemon")
                .takes_value(false),
        )
}
