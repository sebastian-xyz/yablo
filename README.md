# YABLO
**Y**et **A**nother **B**attery **L**ife **O**ptimizer for Linux 🐧

Yablo reduces the energy consumption of the CPU by automatically setting the CPU governor and Turbo Boost dependend on the battery state and system load.
It is highly inspired by [auto-cpufreq](https://github.com/AdnanHodzic/auto-cpufreq).
The application is written in Rust.

![Running yablo daemon](images/yablo_daemon.png "Running yablo daemon")

### Features
- automatically sets CPU governor dependend on battery state and load
- automatically activates or deactivates Turbo Boost dependend on battery state and load
- saves energy by reducing the power consumption by the CPU

### CLI
Available flags
- `-d`, `--debug`:  Shows debug/system info
- `-h`, `--help`: Prints help information
- `-l`, `--live`: Prints information and applies suggested CPU optimizations
- `--log`: View live CPU optimization log made by daemon
- `-m`, `--monitor`: Suggests CPU optimizations for the current load
- `-u`, `--update-config`: Reloads the systemd daemon 
- `-V`, `--version`: Prints version information

## Config

The configuration file is stored at `/etc/yablo/config.toml`.
It allows setting the following parameters separately for the battery states `plugged_in` and `on_battery`

### On AC power (`plugged_in`)
- `governor` (*String*): sets the default governor for low system load
- `turbo` (*Bool*): allows disabling Turbo Boost to save energy
- `second_stage_governor` (*String*): sets a different governor for high system load or CPU usage. Can be used to compensate for a disabled turbo boost when running on battery (optional, default: `performance`)
- `turbo_delay` (*Integer*): sets the delay in seconds before the Turbo gets activated (optional, default: 0)
- `loadperc_threshold` (*Float*): sets the threshold value for the cpu load (optional, default: `20.0`)
- `loadavg_threshold` (*Float*): set the threshold value for the load average (optional, default: `(50.0 * num_cpus)/100.0`)

### On Battery power (`on_battery`)
- `governor` (*String*): sets the default governor for low system load
- `turbo` (*Bool*): allows disabling Turbo Boost to save energy
- `second_stage_governor` (*String*): sets a different governor for high system load or CPU usage. Can be used to compensate for a disabled turbo boost when running on battery (optional, default: `powersave`)
- `turbo_delay` (*Integer*): sets the delay in seconds before the Turbo gets activated (optional, default: 0)
- `battery_threshold` (*Integer*): sets the threshold value for the battery capacity to limit the energy consumption on load battery states - turbo completly disabled (optional, default: `0`)
- `low_battery_governor` (*String*): sets the governor that is used on low battery states (optinal, default: `powersave`)
- `loadperc_threshold` (*Float*): sets the threshold value for the cpu load (optional, default: `30.0`)
- `loadavg_threshold` (*Float*): set the threshold value for the load average (optional, default: `(75.0 * num_cpus)/100.0`)

### Default config
```toml
# /etc/yablo/config.toml
[plugged_in]
governor = "performance"
turbo = true

[on_battery]
governor = "powersave"
turbo = true

```

## Installation

### Install script

The easiest way is to use the install script. It checks for system compatability and installs all the components.
It also prompts if you want to start and enable the systemd service to have `yablo` always running.

```bash
git clone https://github.com/sebastian-xyz/yablo.git
cd yablo
sudo ./install_script $(whoami)
```
The install script allows to choose between the compilation of the binary locally or downloading it from GitHub and check the integrity using a hashsum.
It also deploys a remove script in the `PATH` to remove the whole program without manual intervention.
Just run `yablo_remove` everything will be removed.

**Note**: The username is necessary for the installation if the binary is not compiled locally to avoid downloading the binary as root.
