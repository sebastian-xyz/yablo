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
- `-V`, `--version`: Prints version information