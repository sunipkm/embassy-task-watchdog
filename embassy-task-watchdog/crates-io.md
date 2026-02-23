# `embassy-task-watchdog`

A robust, flexible watchdog management library for embedded systems that multiplexes multiple task watchdogs into a single hardware watchdog timer, preventing system lockups when tasks fail to respond.

## Key Features

- **Hardware Agnostic API**: Implements a consistent, asynchronous interface across different embedded microcontrollers by leveraging [`embassy`](https://embassy.dev).
- **Task Multiplexing**: Consolidates multiple independent task watchdogs into a single hardware watchdog, triggering if any task fails to check in.
- **Compile-time Task Management**: The [`embassy_task_watchdog::task`](https://docs.rs/embassy_task_watchdog_macros/latest/embassy_task_watchdog_macros/attr.task.html) macro replaces [`embassy_executor::task`](https://docs.embassy.dev/embassy-executor/git/cortex-m/attr.task.html), and automatically registers the task with the Watchdog.
- **No-alloc Mode**: Functions in `no_alloc` mode for environments without heap availability.
- **Configurable Timeouts**: Individual timeout durations for each registered task.
- **`no_std` Compatible**: Designed for resource-constrained embedded environments without an operating system.
- **Compile-time Total Tasks Check**: By default, the library supports 32 watchdog tasks. The limit can be changed
  by setting the `EMBASSY_TASK_WATCHDOG_MAX_TASKS` variable either in your [`.cargo/config.toml`](examples/task-pico2/.cargo/config.toml), or by passing it as an environment variable to cargo, e.g. `EMBASSY_TASK_WATCHDOG_MAX_TASKS=8 cargo build`.

## Quick Start

Examples are provided for Raspberry Pi series of microcontrollers, as well as the STM32 microcontrollers using [`embassy`](https://embassy.dev). The examples support the Pico, Pico 2 and STM32F103C8 (blue pill).

First, [install Rust](https://www.rust-lang.org/tools/install)

Add the appropriate target(s):

```bash
rustup target add thumbv6m-none-eabi         # RP2040/Pico
rustup target add thumbv8m.main-none-eabihf  # RP235x/Pico 2
rustup target add thumbv7m-none-eabi         # STM32
rustup target add thumbv7em-none-eabi        # NRF
```

Next, [install `probe-rs`](https://probe.rs/docs/getting-started/installation/)

Now connect your Pico/Pico 2/STM32F103C8 device to a connected debug probe, and go into one of:

- For RP2040/Pi Pico: `examples/task-pico`
- For RP235XA/Pi Pico 2: `examples/task-pico2`
- For STM32F103C8 (blue pill): `examples/task-stm32`

Then execute

```bash
cargo run --release
```

To understand how to use `embassy-task-watchdog` yourself, check out one of the examples:

- [`task-pico`](examples/task-pico/src/main.rs) - A very basic Pi Pico async example
- [`task-pico2`](examples/task-pico2/src/main.rs) - A very basic Pi Pico 2 async example
- [`task-stm32`](examples/task-stm32/src/main.rs) - A very basic Blue Pill async example

## Usage

The library supports the embassy-executor asynchronous API.

To use in your project, add the following line to your `Cargo.toml` file:

```toml
embassy-task-watchdog = { version = "0.0.2", features = ["rp"] } # additionally, supports defmt for logging, stm32 for STM32 devices...
```

### Features
- `rp`: For Raspberry Pi Pico (RP2040) / Pico 2 (RP235xA) or RP235xB based devices. Set up for the correct chip in your `Cargo.toml` by selecting the correct feature in the `embassy-rp` dependency.
  - `defmt-embassy-rp`: [`defmt`](https://docs.rs/defmt/latest/defmt/) debugging for the Embassy executors.
- `stm32`: For STM32 series of devices. Set up for the correct chip in your `Cargo.toml` by selecting the correct feature in the `embassy-stm32` dependency.
  - `defmt-embassy-stm32`: `defmt` debugging for Embassy executors.
- `defmt`: Enable `defmt::Format` for the `embassy-task-watchdog` crate.
- `defmg-messages`: Enable log messages from the `embassy-task-watchdog` crate.

### Core Concepts

- **Task Registration**: Each monitored task is registered with its own timeout period. The registration is automatic.
- **Feeding**: Tasks must feed, or pet, the watchdog within their timeout period to prevent a reset.
- **Task Multiplexing**: The library efficiently manages multiple task timeouts through a single hardware watchdog, triggering if any individual task fails to check in.

![Task Watchdog Multiplexing](https://raw.githubusercontent.com/sunipkm/embassy-task-watchdog/refs/heads/master/docs/images/multiplex.svg)

### Asynchronous API using Embassy

Tasks feed the watchdog asynchronously, powered by Embassy:

```Rust
// Setup
let (watchdog, watchdogtask) = create_watchdog!(hw_watchdog, config);

// Spawn the watchdog task itself
spawner.must_spawn(watchdog_task(watchdogtask));

// In your application tasks
#[embassy_task_watchdog::task(timeout = Duration::from_millis(2000))]
async fn main_task(watchdog: TaskWatchdog) -> ! {
    loop {
        // Do work...
        watchdog.feed().await;
        Timer::after(Duration::from_millis(1000)).await;
    }
}

// Implement other tasks
```

## Platform Support

The crate includes first-class support for:

- RP2040 and RP2350 (Raspberry Pi Pico and Pico 2) via the `rp` feature.
- STM32 family via the `stm32` feature.
- `defmt` for `defmt` based logging.

## License

Licensed under either of the following, at your option:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

## Inspiration

This work is inspired heavily by the [`task-watchdog`](https://github.com/piersfinlayson/task-watchdog)
crate by Piers Finlayson, which provides
a task multiplexing watchdog for embedded systems. It has not been maintained in almost
a year (last commit was on April 10, 2025). This crate is a fork of that work, with the following
goals:

- Update the codebase to be compatible with the latest versions of Rust and Embassy, and to
  use modern Rust features and idioms.
- Automate the task registration process with a procedural macro, to reduce boilerplate and
  make it easier to use.
- Get rid of custom task identifier types through the `Id` trait that had to be manually managed.
- Allow compile-time flexibility without allocations, with as many checks as possible.

To achieve these goals, the codebase has been refactored to limit its scope to
embassy-based async applications, which is the primary use case for this crate. The API has
been redesigned to be more ergonomic and easier to use, while still providing the same core
functionality of multiplexing multiple task watchdogs into a single hardware watchdog timer.
