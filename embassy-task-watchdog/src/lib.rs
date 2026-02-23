//! # embassy-task-watchdog
//!
//! A robust, flexible watchdog management library for embedded systems that
//! multiplexes multiple task watchdogs into a single hardware watchdog timer,
//! preventing system lockups when tasks fail to respond
//!
//! This crate provides a task registration pattern that monitors multiple
//! tasks and ensures they are all still active, feeding the hardware
//! watchdog only if all tasks are healthy.
//!
//!
//! ![Multiplexed Task Diagram](https://raw.githubusercontent.com/piersfinlayson/task-watchdog/refs/heads/main/docs/images/multiplex.svg)
//!
//! ## Key Features
//!
//! - **Task Multiplexing**: Consolidates multiple independent task watchdogs
//!   into a single hardware watchdog, triggering if any task fails to check in
//! - **Static and Automated Task Management**: Tasks are registered at compile-time,
//!   allowing hassle-free integration without dynamic memory allocation, and with
//!   minimal boilerplate using the provided `#[task]` macro.  By default, the library
//!   supports 32 watchdog tasks. The limit can be changed by setting the
//!   `EMBASSY_TASK_WATCHDOG_MAX_TASKS` variable either in
//!   [your `.cargo/config.toml`](https://github.com/sunipkm/embassy-task-watchdog/blob/master/examples/task-pico2/.cargo/config.toml), or by passing
//!   it as an environment variable to cargo, e.g. `EMBASSY_TASK_WATCHDOG_MAX_TASKS=8 cargo build`.
//!   The check is disabled in debug builds to prevent errors in IDEs, but exceeding the
//!   number of tasks will trigger a compiler error in the release build.
//! - **Async Support**: Works with asynchronous (Embassy) execution environments
//! - **Configurable Timeouts**: Individual timeout durations for each
//!   registered task
//! - **`no_std` Compatible**: Designed for resource-constrained embedded
//!   environments without an operating system
//!
//! ## Usage
//!
//! The following is a complete, minimal, example for using the task-watchdog
//! crate using embassy-rs on an RP2040 or RP2350 (Pico or Pico 2).
//! It uses static allocation (no alloc), and creates two tasks with
//! different timeouts, both of which are policed by task-watchdog, and in
//! turn, the hardware watchdog.
//!
//! ```rust
//! # #![no_std]
//! # #![no_main]
//! # use defmt_rtt as _;
//! # use embassy_executor::Spawner;
//! # use embassy_rp::config::Config;
//! # use embassy_task_watchdog::{
//! #     WatchdogConfig, create_watchdog,
//! #     embassy_rp::{TaskWatchdog, WatchdogRunner, watchdog_run},
//! # };
//! # use embassy_time::{Duration, Timer};
//! # use panic_probe as _;
//! # use static_cell::StaticCell;
//!
//! #[embassy_executor::main]
//! async fn main(spawner: Spawner) {
//!     // Initialize the hardare peripherals
//!     let p = embassy_rp::init(Config::default());
//!     // Create the watchdog runner, store it in a static cell, and get the watchdog and watchdog runner task.
//!     let (watchdog, watchdogtask) = create_watchdog!(p.WATCHDOG, config);
//!     // Spawn tasks that will feed the watchdog
//!     spawner.must_spawn(main_task(watchdog));
//!     spawner.must_spawn(second_task(watchdog));
//!     // Finally spawn the watchdog - this will start the hardware watchdog, and feed it
//!     // for as long as _all_ tasks are healthy.
//!     spawner.must_spawn(watchdog_task(watchdogtask));
//! }
//! // Provide a simple embassy task for the watchdog
//! #[embassy_executor::task]
//! async fn watchdog_task(watchdog: WatchdogRunner) -> ! {
//!     watchdog_run(watchdog).await
//! }
//! // Implement your main task
//! #[embassy_task_watchdog::task(timeout = Duration::from_millis(1500))]
//! async fn main_task(watchdog: TaskWatchdog) -> ! {
//!     loop {
//!         // Feed the watchdog
//!         watchdog.feed().await;
//!         // Do some work
//!         Timer::after(Duration::from_millis(1000)).await;
//!     }
//! }
//! // Implement your second task
//! #[embassy_task_watchdog::task(timeout = Duration::from_millis(2000))]
//! async fn second_task(watchdog: TaskWatchdog) -> ! {
//!     loop {
//!         // Feed the watchdog
//!         watchdog.feed().await;
//!         // Do some work
//!         Timer::after(Duration::from_millis(2000)).await;
//!     }
//! }
//! ```
//! See the [examples](https://github.com/sunipkm/embassy-task-watchdog/tree/master/examples)
//! for more usage examples.
//!
//! ## Targets
//!
//! For embedded devices you need to install and specify your target when
//! building.  Use:
//! - RP2040 - `thumbv6m-none-eabi`
//! - RP2350 - `thumbv8m.main-none-eabihf`
//!
//! ## Feature Flags
//!
//! The following feature flags are supported
//!
//! - `rp`: Enable the Raspberry Pi MCU-specific embassy implementation
//! - `defmt-embassy-rp`: Enable logging with defmt for the RP2040 and RP2350 embassy
//! - `stm32`: Enable the STM32 MCU-specific embassy implementation
//! - `defmt-embassy-stm32`: Enable logging with defmt for the STM32 embassy
//! - `defmt`: Enable [`defmt`] logging of associated structs and enums.
//! - `defmt-messages`: Enable `defmt` logging of events and errors in the library.
//!
//! ### Example Feature/Target combination
//!
//! This builds the library for RP2040 with embassy and defmt support:
//!
//! ```bash
//! cargo build --features rp,defmt-embassy-rp --target thumbv6m-none-eabi
//! ```
//! #### Note
//! It is recommended to build the project and run it by writing the build configuration
//! in `.cargo/config.toml`, and executing `cargo build` without any additional
//! arguments.
//!
//! ### Inspiration
//! This work is inspired heavily by the `task-watchdog` crate by Piers Finlayson, which provides
//! a similar task multiplexing watchdog for embedded systems. It has not been maintained in almost
//! a year (last commit was on April 10, 2025). This crate is a fork of that work, with the following
//! goals:
//! - Update the codebase to be compatible with the latest versions of Rust and Embassy, and to
//!   use modern Rust features and idioms.
//! - Automate the task registration process with a procedural macro, to reduce boilerplate and
//!   make it easier to use.
//! - Get rid of custom task identifier types through the `task_watchdog::Id` trait.
//!
//! To achieve these goals, the codebase has been refactored and the scope has been limited to
//! embassy-based async applications, which is the primary use case for this crate.  The API has
//! been redesigned to be more ergonomic and easier to use, while still providing the same core
//! functionality of multiplexing multiple task watchdogs into a single hardware watchdog timer.
//!
// Copyright (c) 2026 Sunip K. Mukherjee <sunipkmukherjee@gmail.com>
//
// Apache 2.0 or MIT licensed, at your option.

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod runtime;
use embassy_time::Duration;
#[doc(hidden)]
pub use runtime::TaskDesc;

pub use embassy_task_watchdog_macros::task;

#[cfg(feature = "defmt-messages")]
#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

// A replacement for the defmt logging macros, when defmt is not provided
#[cfg(not(feature = "defmt-messages"))]
mod log_impl {
    #![allow(unused_macros)]
    #![allow(unused_imports)]
    // Macros are defined as _ to avoid conflicts with built-in attribute
    // names
    macro_rules! _trace {
        ($($arg:tt)*) => {};
    }
    macro_rules! _debug {
        ($($arg:tt)*) => {};
    }
    macro_rules! _info {
        ($($arg:tt)*) => {};
    }
    macro_rules! _warn {
        ($($arg:tt)*) => {};
    }
    macro_rules! _error {
        ($($arg:tt)*) => {};
    }
    pub(crate) use _debug as debug;
    pub(crate) use _error as error;
    pub(crate) use _info as info;
    pub(crate) use _trace as trace;
    pub(crate) use _warn as warn;
}
#[cfg(not(feature = "defmt-messages"))]
use log_impl::*;

pub(crate) use embassy_task_watchdog_numtasks::MAX_TASKS;

/// Represents a hardware-level watchdog that can be fed and reset the system.
pub trait HardwareWatchdog {
    /// Start the hardware watchdog with the given timeout.
    fn start(&mut self, timeout: Duration);

    /// Feed the hardware watchdog to prevent a system reset.
    fn feed(&mut self);

    /// Trigger a hardware reset.
    fn trigger_reset(&mut self) -> !;

    /// Get the reason for the last reset, if available.
    fn reset_reason(&self) -> ResetReason;
}

/// Represents the reason for a system reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ResetReason {
    /// Reset was forced by software.
    Forced,

    /// Reset was caused by watchdog timeout.
    TimedOut,

    /// Reset was caused by an unknown reason.
    Unknown,

    /// No reset has occurred since the last time the reason was cleared.
    None,
}

/// Configuration for the watchdog.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WatchdogConfig {
    /// Timeout to start the hardware watchdog with.
    pub(crate) hardware_timeout: Duration,

    /// Interval at which to check if tasks have fed the watchdog.  Must be
    /// less than the hardware timeout, or the hardware watchdog will reset
    /// the system, before the task-watchdog has a chance to check tasks and
    /// feed it.
    pub(crate) check_interval: Duration,
}

impl WatchdogConfig {
    /// Create a new configuration with specified timeout values
    pub fn new(hardware_timeout: Duration, check_interval: Duration) -> Self {
        Self {
            hardware_timeout,
            check_interval,
        }
    }

    /// Create a default configuration with standard timeout values:
    /// - Hardware timeout: 5000ms
    /// - Check interval: 1000ms
    fn default() -> Self {
        Self::new(Duration::from_millis(5000), Duration::from_millis(1000))
    }
}

impl Default for WatchdogConfig {
    /// Create a default configuration with standard timeout values:
    /// - Hardware timeout: 5000ms
    /// - Check interval: 1000ms
    fn default() -> Self {
        Self::default()
    }
}

/// Errors that can occur when interacting with the watchdog.
pub enum Error {
    /// No slots available to register a task.
    NoSlotsAvailable,
}

mod impl_macro;

/// An async implementation of embassy-task-watchdog for use with the RP2040 and RP2350
/// embassy implementations.
///
/// This module requires the `rp` feature flag to be enabled.
///
/// The main entrypoint into this module is the [`create_watchdog`] macro, which returns
/// the [`embassy_rp::TaskWatchdog`] passed to the tasks, and the [`embassy_rp::WatchdogRunner`] passed to the
/// [`embassy_rp::watchdog_run`] function.  See the documentation for that macro for more details and an example.
///
/// There is an equivalent `embassy_stm32` module for STM32, enabled by
/// the `stm32` feature flag.
#[cfg(feature = "rp")]
#[cfg_attr(docsrs, doc(cfg(feature = "rp")))]
pub mod embassy_rp;

/// An async implementation of embassy-task-watchdog for use with the STM32
/// embassy implementations.
///
/// This module requires the `stm32` feature flag to be enabled.
///
/// The main entrypoint into this module is the [`create_watchdog`] macro, which returns
/// the [`embassy_stm32::TaskWatchdog`] passed to the tasks, and the [`embassy_stm32::WatchdogRunner`] passed to the
/// [`embassy_stm32::watchdog_run`] function.  See the documentation for that macro for more details and an example.
///
/// There is an equivalent `embassy_rp` module for RP2040 and RP2350, enabled by
/// the `rp` feature flag.
#[cfg(feature = "stm32")]
#[cfg_attr(docsrs, doc(cfg(feature = "stm32")))]
pub mod embassy_stm32;

/// Initialize the static memory for the watchdog, and return the watchdog and
/// the watchdog runner task. Pass the [`TaskWatchdog` struct](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_rp/struct.RpTaskWatchdog.html)
/// to your tasks to be able to feed the watchdog. Pass the
/// [`WatchdogRunner` struct](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_rp/struct.RpWatchdogRunner.html)
/// to the [`watchdog_run` function](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_rp/fn.watchdog_run.html)
/// inside a spawned task to monitor the tasks and feed the hardware watchdog.
#[cfg(all(feature = "rp", not(feature = "stm32")))]
#[macro_export]
macro_rules! create_watchdog {
    ($wdt: expr, $config: expr) => {{
        use $crate::embassy_rp::Watchdog;
        // Create a static to hold the task-watchdog object, so it has static
        // lifetime and can be shared with tasks.
        static WATCHDOG: static_cell::StaticCell<Watchdog> = static_cell::StaticCell::new();
        // Create the watchdog runner and store it in the static cell
        let watchdog = Watchdog::new($wdt, $config);
        WATCHDOG.init(watchdog).build()
    }};
}

#[macro_export]
#[cfg(all(feature = "stm32", not(feature = "rp")))]
/// Initialize the static memory for the watchdog, and return the watchdog and
/// the watchdog runner task. Pass the [`TaskWatchdog` struct](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_stm32/struct.Stm32TaskWatchdog.html)
/// to your tasks to be able to feed the watchdog. Pass the
/// [`WatchdogRunner` struct](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_stm32/struct.Stm32WatchdogRunner.html)
/// to the [`watchdog_run` function](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_stm32/fn.watchdog_run.html)
/// inside a spawned task to monitor the tasks and feed the hardware watchdog.
macro_rules! create_watchdog {
    ($wdt: expr, $config: expr) => {{
        use $crate::embassy_stm32::Watchdog;
        // Create a static to hold the task-watchdog object, so it has static
        // lifetime and can be shared with tasks.
        static WATCHDOG: static_cell::StaticCell<Watchdog> = static_cell::StaticCell::new();
        // Create the watchdog runner and store it in the static cell
        let watchdog = Watchdog::new($wdt, $config);
        WATCHDOG.init(watchdog).build()
    }};
}

#[cfg(all(feature = "stm32", feature = "rp"))]
#[macro_export]
/// Initialize the static memory for the watchdog, and return the watchdog and
/// the watchdog runner task. Pass the [`TaskWatchdog` struct](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_rp/struct.RpTaskWatchdog.html)
/// to your tasks to be able to feed the watchdog. Pass the
/// [`WatchdogRunner` struct](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_rp/struct.RpWatchdogRunner.html)
/// to the [`watchdog_run` function](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_rp/fn.watchdog_run.html)
/// inside a spawned task to monitor the tasks and feed the hardware watchdog.
macro_rules! create_watchdog {
    ($wdt: expr, $config: expr) => {
        compile_error!("Cannot use create_watchdog macro with both rp and stm32 features enabled. Please choose one or the other.")
    };
}
