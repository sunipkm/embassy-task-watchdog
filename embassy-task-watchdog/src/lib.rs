//! # task-watchdog
//!
//! A robust, flexible watchdog management library for embedded systems that
//! multiplexes multiple task watchdogs into a single hardware watchdog timer,
//! preventing system lockups when tasks fail to respond
//!
//! This crate provides a task registration pattern that monitors multiple
//! tasks and ensures they are all still active, feeding the hardware
//! watchdog only if all tasks are healthy.
//!
//! Tasks can be dynamically registered and deregistered when the system is
//! running, to allow tasks that are created after startup to be monitoring,
//! and to prevent tasks that are expected to block/pause from causing the
//! device to restart.
//!
//! ![Multiplexed Task Diagram](https://raw.githubusercontent.com/piersfinlayson/task-watchdog/refs/heads/main/docs/images/multiplex.svg)
//!
//! ## Key Features
//!
//! - **Task Multiplexing**: Consolidates multiple independent task watchdogs
//!   into a single hardware watchdog, triggering if any task fails to check in
//! - **Dynamic Task Management**: Tasks can be registered and deregistered
//!   at runtime, allowing for flexible monitoring configurations
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
//! #![no_std]
//! #![no_main]
//!
//! use task_watchdog::{WatchdogConfig, Id};
//! use task_watchdog::embassy_rp::{WatchdogRunner, watchdog_run};
//! use embassy_time::{Duration, Timer};
//! use embassy_rp::config::Config;
//! use embassy_executor::Spawner;
//! use static_cell::StaticCell;
//! use panic_probe as _;
//!
//! // Create a static to hold the task-watchdog object, so it has static
//! // lifetime and can be shared with tasks.
//! static WATCHDOG: StaticCell<WatchdogRunner<TaskId, NUM_TASKS>> = StaticCell::new();
//!
//! // Create an object to contain our task IDs.  It must implement the Id
//! // trait, which, for simply TaskId types means deriving the following
//! // traits:
//! #[derive(Clone, Copy, PartialEq, Eq, Debug)]
//! enum TaskId {
//!     Main,
//!     Second,
//! }
//! impl Id for TaskId {}  // Nothing else to implement as we derived the required traits
//! const NUM_TASKS: usize = 2;
//!
//! #[embassy_executor::main]
//! async fn main(spawner: Spawner) {
//!     // Initialize the hardare peripherals
//!     let p = embassy_rp::init(Config::default());
//!
//!     // Set up watchdog configuration, with a 5s hardware watchdog timeout, and
//!     // with the task watchdog checking tasks every second.
//!     let config = WatchdogConfig {
//!         hardware_timeout: Duration::from_millis(5000),
//!         check_interval: Duration::from_millis(1000),
//!     };
//!
//!     // Create the watchdog runner and store it in the static cell
//!     let watchdog = WatchdogRunner::new(p.WATCHDOG, config);
//!     let watchdog = WATCHDOG.init(watchdog);
//!
//!     // Register our tasks with the task-watchdog.  Each can have a different timeout.
//!     watchdog.register_task(&TaskId::Main, Duration::from_millis(2000)).await;
//!     watchdog.register_task(&TaskId::Second, Duration::from_millis(4000)).await  ;
//!
//!     // Spawn tasks that will feed the watchdog
//!     spawner.must_spawn(main_task(watchdog));
//!     spawner.must_spawn(second_task(watchdog));
//!
//!     // Finally spawn the watchdog - this will start the hardware watchdog, and feed it
//!     // for as long as _all_ tasks are healthy.
//!     spawner.must_spawn(watchdog_task(watchdog));
//! }
//!
//! // Provide a simple embassy task for the watchdog
//! #[embassy_executor::task]
//! async fn watchdog_task(watchdog: &'static WatchdogRunner<TaskId, NUM_TASKS>) -> ! {
//!     watchdog_run(watchdog.create_task()).await
//! }
//!
//! // Implement your main task
//! #[embassy_executor::task]
//! async fn main_task(watchdog: &'static WatchdogRunner<TaskId, NUM_TASKS>) -> !{
//!    loop {
//!         // Feed the watchdog
//!         watchdog.feed(&TaskId::Main).await;
//!
//!         // Do some work
//!         Timer::after(Duration::from_millis(1000)).await;
//!    }
//! }
//!
//! // Implement your second task
//! #[embassy_executor::task]
//! async fn second_task(watchdog: &'static WatchdogRunner<TaskId, NUM_TASKS>) -> !{
//!    loop {
//!         // Feed the watchdog
//!         watchdog.feed(&TaskId::Second).await;
//!
//!         // Do some work
//!         Timer::after(Duration::from_millis(2000)).await;
//!    }
//! }
//!
//! ```
//! See the [`README`](https://github.com/piersfinlayson/task-watchdog/blob/main/README.md) and the [examples](https://github.com/piersfinlayson/task-watchdog/tree/main/examples/src) for more usage examples.
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
//! ### Embassy support:
//!
//! - `rp2040`: Enable the RP2040-specific embassy implementation
//! - `rp235xa`: Enable the RP235Xa-specific embassy implementation
//! - `rp235xb`: Enable the RP235Xb-specific embassy implementation
//! - `defmt-embassy-rp`: Enable logging with defmt for the RP2040 and RP2350 embassy
//!
//! ### Example Feature/Target combination
//!
//! This builds the library for RP2040 with embassy and defmt support:
//!
//! ```bash
//! cargo build --features rp2040,defmt-embassy-rp --target thumbv6m-none-eabi
//! ```
//!
//! ## Embassy Objects
//!
//! If you want to use an include, off the shelf implementation that works with
//! Embassy the objects, you need to use are:
//!
//! - [`WatchdogConfig`] - Used to configure the task-watchdog.
//! - [`embassy_rp::WatchdogRunner`] - Create with the hardware watchdog
//!   peripheral and `WatchdogConfig`, and then use to operate the task-watchdog, including task management.  There is also an `embassy_stm32::WatchdogRunner` for STM32, and `embassy_nrf::WatchdogRunner` for nRF.
//! - [`Id`] - Trait for task identifiers.  If you use an enum, derive the
//!   [`Clone`], [`Copy`], [`PartialEq`], [`Eq`] and [`Debug`]/[`defmt::Format`] traits, and then
//!   implement [`Id`] for the enum.  The Id implementation can be empty, if you
//!   derive the required implementations.  
//! - [`embassy_rp::watchdog_run()`] - Create and spawn a simple embassy task
//!   that just calls this function.  This task will handle policing your other
//!   tasks and feeding the hardware watchdog.
//!

// Copyright (c) 2025 Piers Finlayson <piers@piers.rocks>
//
// Apache 2.0 or MIT licensed, at your option.

#![no_std]
#![warn(missing_docs)]

mod runtime;
#[doc(hidden)]
pub use runtime::TaskDesc;
#[doc(hidden)]
pub(crate) use runtime::TaskKey;

pub use embassy_task_watchdog_macros::task;

#[cfg(feature = "defmt")]
#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

// A replacement for the defmt logging macros, when defmt is not provided
#[cfg(not(feature = "defmt"))]
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
#[cfg(not(feature = "defmt"))]
use log_impl::*;

mod config {
    #![allow(unused)]
    include!(concat!(env!("OUT_DIR"), "/config.rs"));
}
pub(crate) use crate::config::MAX_TASKS;

/// Represents a hardware-level watchdog that can be fed and reset the system.
pub trait HardwareWatchdog {
    /// Start the hardware watchdog with the given timeout.
    fn start(&mut self, timeout: embassy_time::Duration);

    /// Feed the hardware watchdog to prevent a system reset.
    fn feed(&mut self);

    /// Trigger a hardware reset.
    fn trigger_reset(&mut self) -> !;

    /// Get the reason for the last reset, if available.
    fn reset_reason(&self) -> Option<ResetReason>;
}

/// Represents the reason for a system reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ResetReason {
    /// Reset was forced by software.
    Forced,

    /// Reset was caused by watchdog timeout.
    TimedOut,
}

/// Configuration for the watchdog.
#[derive(Debug, Clone, Copy)]
pub struct WatchdogConfig {
    /// Timeout to start the hardware watchdog with.
    pub hardware_timeout: embassy_time::Duration,

    /// Interval at which to check if tasks have fed the watchdog.  Must be
    /// less than the hardware timeout, or the hardware watchdog will reset
    /// the system, before the task-watchdog has a chance to check tasks and
    /// feed it.
    pub check_interval: embassy_time::Duration,
}

impl WatchdogConfig {
    /// Create a new configuration with specified timeout values
    pub fn new(
        hardware_timeout: embassy_time::Duration,
        check_interval: embassy_time::Duration,
    ) -> Self {
        Self {
            hardware_timeout,
            check_interval,
        }
    }

    /// Create a default configuration with standard timeout values:
    /// - Hardware timeout: 5000ms
    /// - Check interval: 1000ms
    fn default() -> Self {
        Self::new(
            embassy_time::Duration::from_millis(5000),
            embassy_time::Duration::from_millis(1000),
        )
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
/// embassy implementations.  There are also stm32 and nRF equivalents of this
/// module.
///
/// This module requires the `rp` feature flag to be enabled.
///
/// There is an equivalent `embassy_stm32` module for STM32, enabled by
/// the `stm32` feature flag, and an `embassy_nrf` module for nRF, enabled by the
/// `nrf` feature flag.
#[cfg(feature = "rp")]
pub mod embassy_rp;

/// An async implementation of embassy-task-watchdog for use with the RP2040 and RP2350
/// embassy implementations.  There are also stm32 and nRF equivalents of this
/// module.
///
/// This module requires the `stm32` feature flag to be enabled.
///
/// There is an equivalent `embassy_rp` module for RP2040 and RP2350, enabled by
/// the `rp` feature flag, and an `embassy_nrf` module for nRF, enabled by the
/// `nrf` feature flag.
#[cfg(feature = "stm32")]
pub mod embassy_stm32;
