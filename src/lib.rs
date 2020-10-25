use nix::unistd::Uid;
use serde_derive::Deserialize;
use systemstat::{Platform, System};

use crossterm::style::Colorize;
use crossterm::ExecutableCommand;
use rev_lines::RevLines;
use std::io::Write;

const TIME_INCREMENT_PER_RUN: u32 = 4;
/* 
    Config related functions and structs
*/

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    pub plugged_in: Option<PowerConfigAC>,
    pub on_battery: Option<PowerConfigBat>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PowerConfigAC {
    pub governor: Option<String>,
    pub turbo: Option<bool>,
    #[serde(default = "default_second_stage_governor_plugged_in")]
    pub second_stage_governor: Option<String>,
    #[serde(default = "default_turbo_delay_governor_plugged_in")]
    pub turbo_delay: Option<u32>,
    #[serde(default = "default_loadperc_threshold_plugged_in")]
    pub loadperc_threshold: Option<f32>,
    #[serde(default = "default_loadavg_threshold_plugged_in")]
    pub loadavg_threshold: Option<f32>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PowerConfigBat {
    pub governor: Option<String>,
    pub turbo: Option<bool>,
    #[serde(default = "default_second_stage_governor_on_battery")]
    pub second_stage_governor: Option<String>,
    #[serde(default = "default_turbo_delay_governor_on_battery")]
    pub turbo_delay: Option<u32>,
    #[serde(default = "default_battery_threshold")]
    pub battery_threshold: Option<u8>,
    #[serde(default = "default_low_battery_governor")]
    pub low_battery_governor: Option<String>,
    #[serde(default = "default_loadperc_threshold_on_battery")]
    pub loadperc_threshold: Option<f32>,
    #[serde(default = "default_loadavg_threshold_on_battery")]
    pub loadavg_threshold: Option<f32>,
}

pub fn check_config_existence() {
    let config_path = "/etc/yablo/";
    match std::fs::metadata(format!("{}{}", config_path, "config.toml")) {
        Ok(_) => {}
        Err(_) => {
            match std::fs::create_dir_all(config_path) {
                Ok(_) => (),
                Err(x) => {
                    eprintln!("[{}] Error: {}", "!".red(), x);
                    std::process::exit(1)
                }
            };
            let default_config = r#"
[plugged_in]
governor = "performance"
turbo = true

[on_battery]
governor = "powersave"
turbo = true



            "#
            .trim();
            match std::fs::write(format!("{}{}", config_path, "config.toml"), default_config) {
                Ok(_) => (),
                Err(x) => {
                    eprintln!("[{}] Error: {}","!".red() , x);
                    std::process::exit(1);
                }
            }
        }
    }
}

pub fn check_config_errors(config: &Config) {
    let avail_govs = get_available_governors();

    if avail_govs.is_empty() {
        eprintln!("[{}] Error: No govenors found. Exit.", "!".red());
        std::process::exit(1)
    }

    if !avail_govs.iter().any(|i| {
        &i.as_str()
            == config
                .plugged_in
                .as_ref()
                .unwrap()
                .governor
                .as_ref()
                .unwrap()
    }) || !avail_govs.iter().any(|i| {
        &i.as_str()
            == config
                .on_battery
                .as_ref()
                .unwrap()
                .governor
                .as_ref()
                .unwrap()
    }) || !avail_govs.iter().any(|i| {
        &i.as_str()
            == config
                .plugged_in
                .as_ref()
                .unwrap()
                .second_stage_governor
                .as_ref()
                .unwrap()
    }) || !avail_govs.iter().any(|i| {
        &i.as_str()
            == config
                .on_battery
                .as_ref()
                .unwrap()
                .second_stage_governor
                .as_ref()
                .unwrap()
    }) {
        eprintln!("[{}] Error: At least one governor specified in config file isn't available!\n     'cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_available_governors' to see available governors", "!".red());
        std::process::exit(1)
    }
}

pub fn get_config() -> Config {
    let decoded_config: Config = match toml::from_str(
        &std::fs::read_to_string("/etc/yablo/config.toml")
            .expect("Something went wrong reading config file"),
    ) {
        Ok(c) => c,
        Err(x) => {
            eprintln!("[{}] Error: {}","!".red() , x);
            std::process::exit(1)
        }
    };
    decoded_config
}

/* 
    System info collection
*/
pub struct SystemInfo {
    pub temperature: f32,
    pub ac_power: bool,
    pub loadavg: f32,
    pub loadperc: f32,
    pub mem_usage: (u64, u64),
    pub turbo_invert: bool,
    pub turbo_avail: bool,
    pub cpu_freqs: Vec<i32>,
    pub battery_capacity: u8,
}

pub fn get_sys_info(sys: &System, turbo_avail: bool, invert: bool, num_cpus: i32) -> SystemInfo {
    SystemInfo {
        loadavg: match sys.load_average() {
            Ok(loadavg) => loadavg.one,
            Err(x) => {
                eprintln!("[{}] Error: {}", "!".red(), x);
                std::process::exit(1)
            }
        },
        temperature: match sys.cpu_temp() {
            Ok(temp) => temp,
            Err(x) => {
                eprintln!("[{}] Error: {}", "!".red(), x);
                std::process::exit(1)
            }
        },
        ac_power: on_ac_power(),
        loadperc: match sys.cpu_load_aggregate() {
            Ok(cpu) => {
                std::thread::sleep(std::time::Duration::from_millis(1000));
                let cpu = cpu.done().unwrap();
                cpu.user * 100.0
                // cpu.system * 100.0
            }
            Err(x) => {
                eprintln!("[{}] Error: {}", "!".red(), x);
                std::process::exit(1)
            }
        },
        turbo_avail: turbo_avail,
        turbo_invert: invert,
        cpu_freqs: get_cpu_freq(num_cpus),
        mem_usage: match sys.memory() {
            Ok(mem) => (mem.total.as_u64(), mem.free.as_u64()),
            Err(x) => {
                eprintln!("[{}] Error: {}", "!".red(), x);
                std::process::exit(1)
            }
        battery_capacity: get_battery_percentage(),
        }
    }
fn get_battery_percentage() -> u8 {
    let bat0_path = "/sys/class/power_supply/BAT0/capacity";
    let bat1_path = "/sys/class/power_supply/BAT1/capacity";
    let bat0_avail = match std::fs::metadata(bat0_path) {
        Ok(_) => true,
        Err(_) => false,
    };
    let bat1_avail = match std::fs::metadata(bat1_path) {
        Ok(_) => true,
        Err(_) => false,
    };
    let batteries = (bat0_avail, bat1_avail);

    match batteries {
        (true, true) => {
            let bat0_capacity = match std::fs::read_to_string(bat0_path) {
                Ok(capacity) => capacity.replace('\n', "").parse::<u8>().unwrap(),
                Err(_) => {
                    eprintln!("[{}] Error: Reading battery capacity for BAT0 (\'/sys/class/power_supply/BAT0/capacity\')", "!".red());
                    std::process::exit(1);
                }
            };
            let bat1_capacity = match std::fs::read_to_string(bat1_path) {
                Ok(capacity) => capacity.replace('\n', "").parse::<u8>().unwrap(),
                Err(_) => {
                    eprintln!("[{}] Error: Reading battery capacity for BAT1 (\'/sys/class/power_supply/BAT1/capacity\')", "!".red());
                    std::process::exit(1);
                }
            };
            return (bat0_capacity + bat1_capacity) / 2;
        }
        (true, false) => {
            match std::fs::read_to_string(bat0_path) {
                Ok(capacity) => return capacity.replace('\n', "").parse::<u8>().unwrap(),
                Err(_) => {
                    eprintln!("[{}] Error: Reading battery capacity for BAT0 (\'/sys/class/power_supply/BAT0/capacity\')", "!".red());
                    std::process::exit(1);
                }
            };
        }
        (false, true) => {
            match std::fs::read_to_string(bat1_path) {
                Ok(capacity) => return capacity.replace('\n', "").parse::<u8>().unwrap(),
                Err(_) => {
                    eprintln!("[{}] Error: Reading battery capacity for BAT1 (\'/sys/class/power_supply/BAT1/capacity\')", "!".red());
                    std::process::exit(1);
                }
            };
        }
        (false, false) => {
            return 101;
        }
    }
}
}

fn on_ac_power() -> bool {
    let bat0_path = "/sys/class/power_supply/BAT0/status";
    let bat1_path = "/sys/class/power_supply/BAT1/status";
    let bat0_avail = match std::fs::metadata(bat0_path) {
        Ok(_) => true,
        Err(_) => false,
    };
    let bat1_avail = match std::fs::metadata(bat1_path) {
        Ok(_) => true,
        Err(_) => false,
    };
    let batteries = (bat0_avail, bat1_avail);

    match batteries {
        (true, true) => {
            match std::fs::read_to_string(bat0_path) {
                Ok(status) => {
                    if status.replace('\n', "").to_lowercase() == "discharging" {
                        return false;
                    } else {
                        match std::fs::read_to_string(bat1_path) {
                            Ok(status) => {
                                if status.replace('\n', "").to_lowercase() == "discharging" {
                                    return false;
                                } else {
                                    return true;
                                }
                            }
                            Err(_) => {
                                eprintln!("[{}] Error: Reading battery status for BAT1 (\'/sys/class/power_supply/BAT1/status\')", "!".red());
                                std::process::exit(1);
                            }
                        };
                    }
                }
                Err(_) => {
                    eprintln!("[{}] Error: Reading battery status for BAT0 (\'/sys/class/power_supply/BAT0/status\')", "!".red());
                    std::process::exit(1);
                }
            };
        }
        (true, false) => {
            match std::fs::read_to_string(bat0_path) {
                Ok(status) => {
                    if status.replace('\n', "").to_lowercase() == "discharging" {
                        return false;
                    } else {
                        return true;
                    }
                }
                Err(_) => {
                    eprintln!("[{}] Error: Reading battery status for BAT0 (\'/sys/class/power_supply/BAT0/status\')", "!".red());
                    std::process::exit(1);
                }
            };
        }
        (false, true) => {
            match std::fs::read_to_string(bat1_path) {
                Ok(status) => {
                    if status.replace('\n', "").to_lowercase() == "discharging" {
                        return false;
                    } else {
                        return true;
                    }
                }
                Err(_) => {
                    eprintln!("[{}] Error: Reading battery status for BAT1 (\'/sys/class/power_supply/BAT1/status\')", "!".red());
                    std::process::exit(1);
                }
            };
        }
        _ => return true,
    }
}

/* 
    Checks
*/

pub fn check_root() {
    if !Uid::effective().is_root() {
        eprintln!("[{}] Error: You have to run this program as root!", "!".red());
        std::process::exit(1)
    }
}

pub fn check_turbo_availability() -> (bool, bool) {
    let p_state = "/sys/devices/system/cpu/intel_pstate/no_turbo";
    let cpufreq = "/sys/devices/system/cpu/cpufreq/boost";

    match std::fs::metadata(p_state) {
        Ok(_) => return (true, true),
        Err(_) => match std::fs::metadata(cpufreq) {
            Ok(_) => return (true, false),
            Err(_) => return (true, false),
        },
    };
}

pub fn check_daemon() {
    let output = std::process::Command::new("systemctl")
        .args(&["is-active", "yablo.service"])
        .output()
        .expect("Failed to execute command");

    if String::from_utf8_lossy(&output.stdout) == "active" {
        eprintln!("[{}] Error: Daemon already installed. Nothing to do. Exit.", "!".red());
        std::process::exit(1)
    }
}

pub fn check_log() {
    let path = "/var/log/yablo.log";
    match std::fs::metadata(path) {
        Ok(_) => (),
        Err(_) => match std::fs::File::create(path) {
            Ok(_) => (),
            Err(x) => {
                eprintln!("[{}] Error: {}", "!".red(), x);
                std::process::exit(1);
            }
        },
    }
}

/*
    Getter und setter
*/

fn get_turbo(invert: bool) -> bool {
    if invert {
        let p_state = "/sys/devices/system/cpu/intel_pstate/no_turbo";
        match std::fs::read_to_string(p_state)
            .expect("Something went wrong reading turbo file")
            .replace('\n', "")
            .as_str()
        {
            "0" => return true,
            "1" => return false,
            _ => panic!("something went wrong"),
        }
    } else {
        let cpufreq = "/sys/devices/system/cpu/cpufreq/boost";
        match std::fs::read_to_string(cpufreq)
            .expect("Something went wrong reading turbo file")
            .replace('\n', "")
            .as_str()
        {
            "0" => return false,
            "1" => return true,
            _ => panic!("something went wrong"),
        }
    }
}

fn set_turbo(new_state: bool, invert: bool) {
    if invert {
        let p_state = "/sys/devices/system/cpu/intel_pstate/no_turbo";
        let output = if new_state { "0" } else { "1" };
        match std::fs::write(p_state, output) {
            Ok(_) => (),
            Err(_) => {
                eprintln!("[{}] Error: couldn't write new turbo state. exit.", "!".red());
                std::process::exit(1);
            }
        }
    } else {
        let cpufreq = "/sys/devices/system/cpu/cpufreq/boost";
        let output = if new_state { "1" } else { "0" };
        match std::fs::write(cpufreq, output) {
            Ok(_) => (),
            Err(_) => {
                eprintln!("[{}] Error: couldn't write new turbo state. exit.", "!".red());
                std::process::exit(1);
            }
        }
    }
}

fn get_available_governors() -> Vec<String> {
    let path = "/sys/devices/system/cpu/cpu0/cpufreq/scaling_available_governors";
    std::fs::read_to_string(path)
        .expect("Something went wrong reading governors file")
        .split(" ")
        .map(|s| s.to_string())
        .collect()
}

fn get_governor() -> String {
    let path = "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor";
    std::fs::read_to_string(path)
        .expect("Something went wrong reading current governor")
        .replace('\n', "")
}

fn set_governor(governor: &str, num_cpus: i32) {
    let path = "/sys/devices/system/cpu/cpu";
    for k in 0..num_cpus {
        match std::fs::write(
            format!("{}{}{}", path, k, "/cpufreq/scaling_governor"),
            governor,
        ) {
            Ok(_) => (),
            Err(x) => {
                eprintln!("[{}] Error: {}. exit.", "!".red(), x);
                std::process::exit(1);
            }
        };
    }
}

fn get_cpu_freq(num_cpus: i32) -> Vec<i32> {
    let path = "/sys/devices/system/cpu/cpu";
    let path_append = "/cpufreq/scaling_cur_freq";
    let mut vec: Vec<i32> = Vec::new();
    for cpu in 0..num_cpus {
        let curr_freq = std::fs::read_to_string(format!("{}{}{}", path, cpu, path_append))
            .expect("Something went wrong reading current frequency")
            .replace('\n', "")
            .parse::<i32>()
            .unwrap()
            / 1000;
        vec.push(curr_freq);
    }
    vec
}

/*
    Printing system info and optimize
*/

pub fn print_info(sys_info: &SystemInfo, terminalout: &mut std::io::Stdout) {
    match terminalout.execute(crossterm::terminal::Clear(
        crossterm::terminal::ClearType::All,
    )) {
        Ok(_) => (),
        Err(x) => {
            eprintln!("[{}] Error: {}", "!".red(), x);
            std::process::exit(1)
        }
    };
    println!("{}", ":".repeat(50));
    println!("{} System state {}", ":".repeat(18), ":".repeat(18));
    println!("{}\n", ":".repeat(50));
    if !sys_info.turbo_avail {
        println!("[{}] No turbo found!", "!".yellow());
    }
    if sys_info.ac_power {
        println!("[{}] Currently running on AC power", "+".green());
    } else {
        println!("[{}] Currently running on battery power", "+".green());
    }
    println!("[{}] CPU temp       : {}°C", "+".green(), sys_info.temperature);
    println!("[{}] Memory usage   : {:.2}GB/{:.2}GB", "+".green(), (sys_info.mem_usage.0 - sys_info.mem_usage.1) as f32/1e9, sys_info.mem_usage.0 as f32/1e9);    
    println!("[{}] System load    : {:.2}", "+".green(), sys_info.loadavg);
    println!("[{}] CPU usage      : {:.2}%", "+".green(), sys_info.loadperc);
    println!("[{}] CPU frequencies: ", "+".green());
    for cpu in 0..sys_info.cpu_freqs.len() {
        println!(
            "    {} CPU{}: {:4}MHz",
            "\u{2218}".blue(),
            cpu,
            sys_info.cpu_freqs[cpu]
        );
    }
    println!("");
    match terminalout.flush() {
        Ok(_) => (),
        Err(x) => {
            eprintln!("[{}] Error: {}", "!".red(), x);
            std::process::exit(1);
        }
    };
}

pub fn optimize_powerstate(
    config: &Config,
    sys_info: &SystemInfo,
    cpus: i32,
    counter: &mut u32,
    terminalout: &mut std::io::Stdout,
) {
    println!("{}", "\u{2591}".repeat(50).blue());
    println!(
        "{} Apply optimizations {}",
        "\u{2591}".repeat(14).blue(),
        "\u{2591}".repeat(15).blue()
    );
    println!("{}\n", "\u{2591}".repeat(50).blue());
    if sys_info.ac_power {
        if sys_info.loadavg
            > config
                .plugged_in
                .as_ref()
                .unwrap()
                .loadavg_threshold
                .unwrap()
        {
            high_load_setting_ac(&config, &sys_info, cpus, counter);
        } else if sys_info.loadperc
            >= config
                .plugged_in
                .as_ref()
                .unwrap()
                .loadperc_threshold
                .unwrap()
        {
            high_load_setting_ac(&config, &sys_info, cpus, counter);
        } else {
            low_load_setting_ac(&config, &sys_info, cpus, counter);
        }
    } else {
        if sys_info.loadavg
            > config
                .on_battery
                .as_ref()
                .unwrap()
                .loadavg_threshold
                .unwrap()
        {
            high_load_setting_bat(&config, &sys_info, cpus, counter);
        } else if sys_info.loadperc
            >= config
                .on_battery
                .as_ref()
                .unwrap()
                .loadperc_threshold
                .unwrap()
        {
            high_load_setting_bat(&config, &sys_info, cpus, counter);
        } else {
            low_load_setting_bat(&config, &sys_info, cpus, counter);
        }
    }
    println!("");
    match terminalout.flush() {
        Ok(_) => (),
        Err(x) => {
            eprintln!("[{}] Error: {}", "!".red(), x);
            std::process::exit(1)
        }
    };
}

pub fn monitor_state(
    config: &Config,
    sys_info: &SystemInfo,
    cpus: i32,
    counter: &mut u32,
    terminalout: &mut std::io::Stdout,
) {
    println!("{}", ":".repeat(50));
    println!("{} Suggest optimzations {}", ":".repeat(14), ":".repeat(14));
    println!("{}\n", ":".repeat(50));
    if sys_info.ac_power {
        if sys_info.loadavg > (50.0 * cpus as f32) / 100.0 {
            println!("[{}] High system load", "+".green());
            println!(
                "[{}] Suggesting use of '{}' governor",
                "+".green(),
                config
                    .plugged_in
                    .as_ref()
                    .unwrap()
                    .second_stage_governor
                    .as_ref()
                    .unwrap()
            );
            println!(
                "[{}] Currently using '{}' governor",
                "+".green(),
                get_governor()
            );
            if config.plugged_in.as_ref().unwrap().turbo.unwrap() {
                *counter = *counter + TIME_INCREMENT_PER_RUN;
                if *counter >= (*config).plugged_in.as_ref().unwrap().turbo_delay.unwrap() {
                    println!("[{}] Suggesting setting Turbo on", "+".green());
                    if get_turbo(sys_info.turbo_invert) {
                        println!("[{}] Turbo is currently on", "+".green());
                    } else {
                        println!("[{}] Turbo is currently off", "+".green());
                    }
                } else {
                    println!("[{}] Suggesting setting Turbo off", "+".green());
                    if get_turbo(sys_info.turbo_invert) {
                        println!("[{}] Turbo is currently on", "+".green());
                    } else {
                        println!("[{}] Turbo is currently off", "+".green());
                    }
                }
            } else {
                println!("[{}] Suggesting setting Turbo off", "+".green());
                if get_turbo(sys_info.turbo_invert) {
                    println!("[{}] Turbo is currently on", "+".green());
                } else {
                    println!("[{}] Turbo is currently off", "+".green());
                }
            }
        } else if sys_info.loadperc >= 20.0 {
            println!("[{}] High CPU usage", "+".green());
            println!(
                "[{}] Suggesting use of '{}' governor",
                "+".green(),
                config
                    .plugged_in
                    .as_ref()
                    .unwrap()
                    .second_stage_governor
                    .as_ref()
                    .unwrap()
            );
            println!(
                "[{}] Currently using '{}' governor",
                "+".green(),
                get_governor()
            );
            if config.plugged_in.as_ref().unwrap().turbo.unwrap() {
                *counter = *counter + TIME_INCREMENT_PER_RUN;
                if *counter >= (*config).plugged_in.as_ref().unwrap().turbo_delay.unwrap() {
                    println!("[{}] Suggesting setting Turbo on", "+".green());
                    if get_turbo(sys_info.turbo_invert) {
                        println!("[{}] Turbo is currently on", "+".green());
                    } else {
                        println!("[{}] Turbo is currently off", "+".green());
                    }
                } else {
                    println!("[{}] Suggesting setting Turbo off", "+".green());
                    if get_turbo(sys_info.turbo_invert) {
                        println!("[{}] Turbo is currently on", "+".green());
                    } else {
                        println!("[{}] Turbo is currently off", "+".green());
                    }
                }
            } else {
                println!("[{}] Suggesting setting Turbo off", "+".green());
                if get_turbo(sys_info.turbo_invert) {
                    println!("[{}] Turbo is currently on", "+".green());
                } else {
                    println!("[{}] Turbo is currently off", "+".green());
                }
            }
        } else {
            println!("[{}] Load optimal", "+".green());
            println!(
                "[{}] Suggesting use of '{}' governor",
                "+".green(),
                config
                    .plugged_in
                    .as_ref()
                    .unwrap()
                    .governor
                    .as_ref()
                    .unwrap()
            );
            println!(
                "[{}] Currently using '{}' governor",
                "+".green(),
                get_governor()
            );
            println!("[{}] Suggesting setting Turbo off", "+".green());
            if get_turbo(sys_info.turbo_invert) {
                println!("[{}] Turbo is currently on", "+".green());
            } else {
                println!("[{}] Turbo is currently off", "+".green());
            }
            *counter = 0;
        }
    } else {
        if sys_info.loadavg > (75.0 * cpus as f32) / 100.0 {
            println!("[{}] High system load", "+".green());
            println!(
                "[{}] Suggesting use of '{}' governor",
                "+".green(),
                config
                    .on_battery
                    .as_ref()
                    .unwrap()
                    .second_stage_governor
                    .as_ref()
                    .unwrap()
            );
            println!(
                "[{}] Currently using '{}' governor",
                "+".green(),
                get_governor()
            );
            if config.on_battery.as_ref().unwrap().turbo.unwrap() {
                *counter = *counter + TIME_INCREMENT_PER_RUN;
                if *counter >= (*config).on_battery.as_ref().unwrap().turbo_delay.unwrap() {
                    println!("[{}] Suggesting setting Turbo on", "+".green());
                    if get_turbo(sys_info.turbo_invert) {
                        println!("[{}] Turbo is currently on", "+".green());
                    } else {
                        println!("[{}] Turbo is currently off", "+".green());
                    }
                } else {
                    println!("[{}] Suggesting setting Turbo off", "+".green());
                    if get_turbo(sys_info.turbo_invert) {
                        println!("[{}] Turbo is currently on", "+".green());
                    } else {
                        println!("[{}] Turbo is currently off", "+".green());
                    }
                }
            } else {
                println!("[{}] Suggesting setting Turbo off", "+".green());
                if get_turbo(sys_info.turbo_invert) {
                    println!("[{}] Turbo is currently on", "+".green());
                } else {
                    println!("[{}] Turbo is currently off", "+".green());
                }
            }
        } else if sys_info.loadperc >= 30.0 {
            println!("[{}] High CPU usage", "+".green());
            println!(
                "[{}] Using '{}' governor",
                "+".green(),
                config
                    .on_battery
                    .as_ref()
                    .unwrap()
                    .second_stage_governor
                    .as_ref()
                    .unwrap()
            );
            if config.on_battery.as_ref().unwrap().turbo.unwrap() {
                *counter = *counter + TIME_INCREMENT_PER_RUN;
                if *counter >= (*config).on_battery.as_ref().unwrap().turbo_delay.unwrap() {
                    println!("[{}] Suggesting setting Turbo on", "+".green());
                    if get_turbo(sys_info.turbo_invert) {
                        println!("[{}] Turbo is currently on", "+".green());
                    } else {
                        println!("[{}] Turbo is currently off", "+".green());
                    }
                } else {
                    println!("[{}] Suggesting setting Turbo off", "+".green());
                    if get_turbo(sys_info.turbo_invert) {
                        println!("[{}] Turbo is currently on", "+".green());
                    } else {
                        println!("[{}] Turbo is currently off", "+".green());
                    }
                }
            } else {
                println!("[{}] Suggesting setting Turbo off", "+".green());
                if get_turbo(sys_info.turbo_invert) {
                    println!("[{}] Turbo is currently on", "+".green());
                } else {
                    println!("[{}] Turbo is currently off", "+".green());
                }
            }
        } else {
            println!("[{}] Load optimal", "+".green());
            println!(
                "[{}] Suggesting use of '{}' governor",
                "+".green(),
                config
                    .on_battery
                    .as_ref()
                    .unwrap()
                    .governor
                    .as_ref()
                    .unwrap()
            );
            println!(
                "[{}] Currently using '{}' governor",
                "+".green(),
                get_governor()
            );
            println!("[{}] Suggesting setting Turbo off", "+".green());
            if get_turbo(sys_info.turbo_invert) {
                println!("[{}] Turbo is currently on", "+".green());
            } else {
                println!("[{}] Turbo is currently off", "+".green());
            }
            *counter = 0;
        }
    }
    println!("");
    match terminalout.flush() {
        Ok(_) => (),
        Err(x) => {
            eprintln!("[{}] Error: {}", "!".red(), x);
            std::process::exit(1)
        }
    };
}

pub fn print_log(num_cpus: i32, terminalout: &mut std::io::Stdout) {
    let log_path = "/var/log/yablo.log";
    let file = std::fs::File::open(log_path).unwrap();
    let rev_lines = RevLines::new(std::io::BufReader::new(file)).unwrap();
    let mut last_lines: Vec<String> = Vec::new();
    let num_lines = num_cpus + 25;
    let mut i = 0;

    for line in rev_lines {
        if i >= num_lines {
            break;
        }
        last_lines.push(line);

        i += 1;
    }
    last_lines.reverse();
    match terminalout.execute(crossterm::terminal::Clear(
        crossterm::terminal::ClearType::All,
    )) {
        Ok(_) => (),
        Err(x) => {
            eprintln!("[{}] Error: {}", "!".red(), x);
            std::process::exit(1)
        }
    };

    for i in last_lines {
        println!("{}", i);
    }
}

    let num_lines = if turbo_avail {
        19 + num_cpus
pub fn restart_daemon() {
    let output = std::process::Command::new("systemctl")
        .args(&["is-active", "yablo.service"])
        .output()
        .expect("Failed to execute command");

    if String::from_utf8_lossy(&output.stdout) == "active\n" {
        let _output = std::process::Command::new("systemctl")
            .args(&["restart", "yablo.service"])
            .output()
            .expect("Failed to execute command");
    } else {
        eprintln!(
            "[{}] Error: Daemon not running. No need to restart daemon to load new config. Exit.",
            "!".red()
        );
        std::process::exit(1)
    }
}

/*

    Setting governor and turbo helper

*/

fn high_load_setting_bat(config: &Config, sys_info: &SystemInfo, cpus: i32, counter: &mut u32) {
    if sys_info.battery_capacity
        > config
            .on_battery
            .as_ref()
            .unwrap()
            .battery_threshold
            .unwrap()
    {
        println!("[{}] High system load", "+".green());
        println!(
            "[{}] Using '{}' governor",
            "+".green(),
            config
                .on_battery
                .as_ref()
                .unwrap()
                .second_stage_governor
                .as_ref()
                .unwrap()
        );
        set_governor(
            config
                .on_battery
                .as_ref()
                .unwrap()
                .second_stage_governor
                .as_ref()
                .unwrap(),
            cpus,
        );
        if config.on_battery.as_ref().unwrap().turbo.unwrap() {
            *counter = *counter + TIME_INCREMENT_PER_RUN;
            if *counter >= (*config).on_battery.as_ref().unwrap().turbo_delay.unwrap() {
                set_turbo(true, sys_info.turbo_invert);
                println!("[{}] Turbo activated", "+".green());
            } else {
                println!("[{}] Turbo deactivated", "+".green());
            }
        } else {
            set_turbo(false, sys_info.turbo_invert);
            println!("[{}] Turbo deactivated", "+".green());
        }
    } else {
        println!("[{}] High system load", "+".green());
        println!("[{}] Low battery capacity", "!".yellow());
        println!(
            "[{}] Using '{}' governor",
            "+".green(),
            config
                .on_battery
                .as_ref()
                .unwrap()
                .low_battery_governor
                .as_ref()
                .unwrap()
        );
        set_governor("powersave", cpus);
        set_turbo(false, sys_info.turbo_invert);
        println!("[{}] Turbo deactivated", "+".green());
    }
}

fn low_load_setting_bat(config: &Config, sys_info: &SystemInfo, cpus: i32, counter: &mut u32) {
    if sys_info.battery_capacity
        > config
            .on_battery
            .as_ref()
            .unwrap()
            .battery_threshold
            .unwrap()
    {
        println!("[{}] Load optimal", "+".green());
        println!(
            "[{}] Using '{}' governor",
            "+".green(),
            config
                .on_battery
                .as_ref()
                .unwrap()
                .governor
                .as_ref()
                .unwrap()
        );
        println!("[{}] Turbo deactivated", "+".green());
        set_governor(
            config
                .on_battery
                .as_ref()
                .unwrap()
                .governor
                .as_ref()
                .unwrap(),
            cpus,
        );
        *counter = 0;
        set_turbo(false, sys_info.turbo_invert);
    } else {
        println!("[{}] Load optimal", "+".green());
        println!("[{}] Low battery capacity", "!".yellow());
        println!(
            "[{}] Using '{}' governor",
            "+".green(),
            config
                .on_battery
                .as_ref()
                .unwrap()
                .low_battery_governor
                .as_ref()
                .unwrap()
        );
        println!("[{}] Turbo deactivated", "+".green());
        set_governor(
            config
                .on_battery
                .as_ref()
                .unwrap()
                .governor
                .as_ref()
                .unwrap(),
            cpus,
        );
        *counter = 0;
        set_turbo(false, sys_info.turbo_invert);
    }
}

fn high_load_setting_ac(config: &Config, sys_info: &SystemInfo, cpus: i32, counter: &mut u32) {
    println!("[{}] High CPU usage", "+".green());
    println!(
        "[{}] Using '{}' governor",
        "+".green(),
        config
            .plugged_in
            .as_ref()
            .unwrap()
            .second_stage_governor
            .as_ref()
            .unwrap()
    );
    set_governor(
        config
            .plugged_in
            .as_ref()
            .unwrap()
            .second_stage_governor
            .as_ref()
            .unwrap(),
        cpus,
    );
    if config.plugged_in.as_ref().unwrap().turbo.unwrap() {
        *counter = *counter + TIME_INCREMENT_PER_RUN;
        if *counter >= (*config).plugged_in.as_ref().unwrap().turbo_delay.unwrap() {
            set_turbo(true, sys_info.turbo_invert);
            println!("[{}] Turbo activated", "+".green());
        } else {
            println!("[{}] Turbo deactivated", "+".green());
        }
    } else {
        set_turbo(false, sys_info.turbo_invert);
        println!("[{}] Turbo deactivated", "+".green());
    }
}

fn low_load_setting_ac(config: &Config, sys_info: &SystemInfo, cpus: i32, counter: &mut u32) {
    println!("[{}] Load optimal", "+".green());
    println!(
        "[{}] Using '{}' governor",
        "+".green(),
        config
            .plugged_in
            .as_ref()
            .unwrap()
            .governor
            .as_ref()
            .unwrap()
    );
    *counter = 0;
    set_governor(
        config
            .plugged_in
            .as_ref()
            .unwrap()
            .governor
            .as_ref()
            .unwrap(),
        cpus,
    );
    println!("[{}] Turbo deactivated", "+".green());
    set_turbo(false, sys_info.turbo_invert);
}

/*
    default values config
*/

fn default_second_stage_governor_plugged_in() -> Option<String> {
    Some(String::from("performance"))
}
fn default_turbo_delay_governor_plugged_in() -> Option<u32> {
    Some(0)
}
fn default_second_stage_governor_on_battery() -> Option<String> {
    Some(String::from("powersave"))
}
fn default_turbo_delay_governor_on_battery() -> Option<u32> {
    Some(0)
}
fn default_battery_threshold() -> Option<u8> {
    Some(0)
}
fn default_low_battery_governor() -> Option<String> {
    Some(String::from("powersave"))
}

fn default_loadperc_threshold_plugged_in() -> Option<f32> {
    Some(20.0)
}

fn default_loadperc_threshold_on_battery() -> Option<f32> {
    Some(30.0)
}

fn default_loadavg_threshold_plugged_in() -> Option<f32> {
    let num_cores = num_cpus::get() as i32;
    Some((50.0 * num_cores as f32) / 100.0)
}

fn default_loadavg_threshold_on_battery() -> Option<f32> {
    let num_cores = num_cpus::get() as i32;
    Some((75.0 * num_cores as f32) / 100.0)
}
