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