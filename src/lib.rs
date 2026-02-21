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
pub(crate) use runtime::TaskKey;
#[doc(hidden)]
pub use runtime::{BoundWatchdog, TaskDesc};

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

/// Represents a hardware-level watchdog that can be fed and reset the system.
pub trait HardwareWatchdog<C: Clock> {
    /// Start the hardware watchdog with the given timeout.
    fn start(&mut self, timeout: C::Duration);

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
pub struct WatchdogConfig<C: Clock> {
    /// Timeout to start the hardware watchdog with.
    pub hardware_timeout: C::Duration,

    /// Interval at which to check if tasks have fed the watchdog.  Must be
    /// less than the hardware timeout, or the hardware watchdog will reset
    /// the system, before the task-watchdog has a chance to check tasks and
    /// feed it.
    pub check_interval: C::Duration,
}

impl<C: Clock> WatchdogConfig<C> {
    /// Create a new configuration with specified timeout values
    pub fn new(hardware_timeout_ms: u64, check_interval_ms: u64, clock: &C) -> Self {
        Self {
            hardware_timeout: clock.duration_from_millis(hardware_timeout_ms),
            check_interval: clock.duration_from_millis(check_interval_ms),
        }
    }

    /// Create a default configuration with standard timeout values:
    /// - Hardware timeout: 5000ms
    /// - Check interval: 1000ms
    pub fn default(clock: &C) -> Self {
        Self::new(5000, 1000, clock)
    }
}

/// Represents a task monitored by the watchdog.
#[derive(Debug, Clone)]
struct Task<C: Clock> {
    /// The task identifier.
    #[allow(dead_code)]
    id: TaskKey,

    /// The last time the task was fed.
    last_feed: C::Instant,

    /// Maximum duration between feeds.
    max_duration: C::Duration,
}

impl<C: Clock> Task<C> {
    /// Creates a new Task object for registration with the watchdog.
    pub fn new(id: TaskKey, max_duration: C::Duration, clock: &C) -> Self {
        Self {
            id,
            last_feed: clock.now(),
            max_duration,
        }
    }

    /// Feed the task to indicate it's still active.
    fn feed(&mut self, clock: &C) {
        self.last_feed = clock.now();
    }

    /// Check if this task has starved the watchdog.
    fn is_starved(&self, clock: &C) -> bool {
        clock.has_elapsed(self.last_feed, &self.max_duration)
    }
}

/// A trait for time-keeping implementations.
pub trait Clock {
    /// A type representing a specific instant in time.
    type Instant: Copy;

    /// A type representing a duration of time
    type Duration: Copy;

    /// Get the current time.
    fn now(&self) -> Self::Instant;

    /// Calculate the duration elapsed since the given instant.
    fn elapsed_since(&self, instant: Self::Instant) -> Self::Duration;

    /// Check if a duration has passed since the given instant.
    fn has_elapsed(&self, instant: Self::Instant, duration: &Self::Duration) -> bool;

    /// Create a duration from milliseconds.
    fn duration_from_millis(&self, millis: u64) -> Self::Duration;
}

/// A Watchdog that monitors multiple tasks and feeds a hardware watchdog accordingly.
struct WatchdogContainer<const N: usize, W, C>
where
    W: HardwareWatchdog<C>,
    C: Clock,
{
    /// The hardware watchdog.
    hw_watchdog: W,

    /// Tasks being monitored.
    tasks: [Option<Task<C>>; N],

    /// Configuration.
    config: WatchdogConfig<C>,

    /// Clock for time-keeping.
    clock: C,
}

/// Errors that can occur when interacting with the watchdog.
pub enum Error {
    /// No slots available to register a task.
    NoSlotsAvailable,
}

impl<W: HardwareWatchdog<C>, C: Clock, const N: usize> WatchdogContainer<N, W, C> {
    /// Create a new watchdog with the given hardware watchdog and configuration.
    ///
    /// Arguments:
    /// * `hw_watchdog` - The hardware watchdog to use.
    /// * `config` - The configuration for the watchdog.
    /// * `clock` - The clock implementation to use for time-keeping.
    fn new(hw_watchdog: W, config: WatchdogConfig<C>, clock: C) -> Self {
        Self {
            hw_watchdog,
            tasks: [const { None }; N],
            config,
            clock,
        }
    }

    /// Register a task with the watchdog.
    ///
    /// The task will be monitored by the watchdog.
    ///
    /// Arguments:
    /// * `id` - The task identifier.
    /// * `max_duration` - The maximum duration between feeds.  If there is
    ///   a gap longer than this, the watchdog will trigger.
    ///
    /// # Errors
    ///
    /// If there are no available slots to register the task, an error will be
    /// returned.
    fn register_task(&mut self, id: &TaskKey, max_duration: C::Duration) -> Result<(), Error> {
        // Find an empty slot
        for slot in &mut self.tasks {
            if slot.is_none() {
                *slot = Some(Task::new(*id, max_duration, &self.clock));
                debug!("Registered task: {:?}", id);
                return Ok(());
            }
        }

        // No empty slots available
        error!("Failed to register task: {:?} - no slots available", id);
        Err(Error::NoSlotsAvailable)
    }

    fn deregister_task(&mut self, id: &TaskKey) {
        for slot in &mut self.tasks {
            if let Some(task) = slot
                && task.id == *id
            {
                *slot = None;
                debug!("Deregistered task: {:?}", id);
                return;
            }
        }
        info!("Attempted to deregister unknown task: {:?}", id);
    }

    fn feed(&mut self, id: &TaskKey) {
        let fed = self.tasks.iter_mut().flatten().any(|task| {
            if task.id == *id {
                task.feed(&self.clock);
                true
            } else {
                false
            }
        });

        if !fed {
            warn!("Attempt to feed unknown task: {:?}", id);
        }
    }

    /// Start the watchdog.
    ///
    /// This starts the hardware watchdog.  You must run the watchdog task
    /// now to monitor the tasks.
    fn start(&mut self) {
        // Feed all registered tasks
        self.tasks.iter_mut().flatten().for_each(|task| {
            task.feed(&self.clock);
        });

        // Start the hardware watchdog
        self.hw_watchdog.start(self.config.hardware_timeout);

        info!("Watchdog started");
    }

    /// Check if any tasks have starved the watchdog and take appropriate action.
    fn check(&mut self) -> bool {
        // Check if any tasks have starved
        let mut starved = false;
        self.tasks.iter_mut().flatten().for_each(|task| {
            if task.is_starved(&self.clock) {
                error!("Task {:?} has starved the watchdog", task.id);
                starved = true;
            }
        });

        // Either feed the hardware watchdog or return that we have a starved
        // task
        if !starved {
            self.hw_watchdog.feed();
        }

        starved
    }

    /// Trigger a system reset.
    fn trigger_reset(&mut self) -> ! {
        warn!("Triggering watchdog reset");
        self.hw_watchdog.trigger_reset()
    }

    /// Get the reason for the last reset.
    pub fn reset_reason(&self) -> Option<ResetReason> {
        self.hw_watchdog.reset_reason()
    }
}

/// A system clock implementation using core time types, which allows
/// task-watchdog to work with different clock implementations.
pub struct CoreClock;

impl Clock for CoreClock {
    type Instant = u64; // Simple millisecond counter
    type Duration = core::time::Duration;

    fn now(&self) -> Self::Instant {
        // In real code, this would use a hardware timer
        // This is just a simple example
        static mut MILLIS: u64 = 0;
        unsafe {
            MILLIS += 1;
            MILLIS
        }
    }

    fn elapsed_since(&self, instant: Self::Instant) -> Self::Duration {
        let now = self.now();
        let elapsed_ms = now.saturating_sub(instant);
        core::time::Duration::from_millis(elapsed_ms)
    }

    fn has_elapsed(&self, instant: Self::Instant, duration: &Self::Duration) -> bool {
        self.elapsed_since(instant) >= *duration
    }

    fn duration_from_millis(&self, millis: u64) -> Self::Duration {
        core::time::Duration::from_millis(millis)
    }
}

/// A system clock implementation for Embassy.
pub struct EmbassyClock;

impl Clock for EmbassyClock {
    type Instant = embassy_time::Instant;
    type Duration = embassy_time::Duration;

    fn now(&self) -> Self::Instant {
        embassy_time::Instant::now()
    }

    fn elapsed_since(&self, instant: Self::Instant) -> Self::Duration {
        embassy_time::Instant::now() - instant
    }

    fn has_elapsed(&self, instant: Self::Instant, duration: &Self::Duration) -> bool {
        (embassy_time::Instant::now() - instant) >= *duration
    }

    fn duration_from_millis(&self, millis: u64) -> Self::Duration {
        embassy_time::Duration::from_millis(millis)
    }
}
/// An async implementation of task-watchdog for use with the RP2040 and RP2350
/// embassy implementations.  There are also stm32 and nRF equivalents of this
/// module.
///
/// This module requires either the `rp2040-embassy` or `rp2350-embassy`
/// feature.
///
/// See the [`embassy`](https://github.com/piersfinlayson/task-watchdog/blob/main/examples/src/embassy.rs)
/// example for how to use this module.
///
/// There is an equivalent `embassy_stm32` module for STM32, but due to
/// docs.rs limitations it is not documented here.  See the above example for
/// usage of that module.  `embassy_nrf` and `embassy_rsp32` also exist.
pub mod embassy_rp {
    use super::{
        Clock, EmbassyClock, HardwareWatchdog, ResetReason, WatchdogConfig, WatchdogContainer, info,
    };
    use embassy_rp::peripherals::WATCHDOG as RpWatchdogPeripheral;
    use embassy_rp::watchdog as rp_watchdog;
    use embassy_time::{Instant, Timer};

    /// RP2040/RP2350-specific watchdog implementation.
    struct RpWatchdog {
        inner: rp_watchdog::Watchdog,
    }

    impl RpWatchdog {
        /// Create a new RP2040/RP2350 watchdog.
        #[must_use]
        pub fn new(peripheral: embassy_rp::Peri<'static, RpWatchdogPeripheral>) -> Self {
            Self {
                inner: rp_watchdog::Watchdog::new(peripheral),
            }
        }
    }

    /// Implement the HardwareWatchdog trait for the RP2040/RP2350 watchdog.
    impl HardwareWatchdog<EmbassyClock> for RpWatchdog {
        fn start(&mut self, timeout: <EmbassyClock as Clock>::Duration) {
            self.inner.start(timeout);
        }

        fn feed(&mut self) {
            self.inner.feed();
        }

        fn trigger_reset(&mut self) -> ! {
            self.inner.trigger_reset();
            panic!("Triggering reset via watchdog failed");
        }

        fn reset_reason(&self) -> Option<ResetReason> {
            self.inner.reset_reason().map(|reason| match reason {
                embassy_rp::watchdog::ResetReason::Forced => ResetReason::Forced,
                embassy_rp::watchdog::ResetReason::TimedOut => ResetReason::TimedOut,
            })
        }
    }

    /// An Embassy RP2040/RP2350 watchdog runner.
    ///
    /// There is an equivalent version of this when using the `alloc` feature
    /// which does not include the `const N: usize` type.
    ///
    /// There is also an equivalent STM32 watchdog runner in the
    /// `embassy_stm32` module.
    ///
    /// Create the watchdog runner using the [`WatchdogRunner::new()`] method, and then use the
    /// methods to register tasks and feed the watchdog.  You probably don't
    /// want to access the other methods directly - use [`watchdog_run()`] to
    /// handle running the task-watchdog.
    pub(crate) struct RpWatchdogOwner<const N: usize> {
        watchdog: embassy_sync::mutex::Mutex<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            core::cell::RefCell<WatchdogContainer<N, RpWatchdog, EmbassyClock>>,
        >,
    }

    impl<const N: usize> RpWatchdogOwner<N> {
        /// Create a new Embassy-compatible watchdog runner.
        pub(crate) fn new(
            hw_watchdog: embassy_rp::Peri<'static, RpWatchdogPeripheral>,
            config: WatchdogConfig<EmbassyClock>,
        ) -> Self {
            let hw_watchdog = RpWatchdog::new(hw_watchdog);
            let watchdog = WatchdogContainer::new(hw_watchdog, config, EmbassyClock);
            Self {
                watchdog: embassy_sync::mutex::Mutex::new(core::cell::RefCell::new(watchdog)),
            }
        }

        /// Register a task with the watchdog.
        pub(crate) async fn register_task(
            &self,
            id: &TaskKey,
            max_duration: <EmbassyClock as Clock>::Duration,
        ) {
            self.watchdog
                .lock()
                .await
                .borrow_mut()
                .register_task(id, max_duration)
                .ok();
        }

        /// Deregister a task with the watchdog.
        pub(crate) async fn deregister_task(&self, id: &TaskKey) {
            self.watchdog.lock().await.borrow_mut().deregister_task(id);
        }

        /// Feed the watchdog for a specific task.
        pub(crate) async fn feed(&self, id: &TaskKey) {
            self.watchdog.lock().await.borrow_mut().feed(id);
        }

        /// Start the watchdog.
        pub(crate) async fn start(&self) {
            self.watchdog.lock().await.borrow_mut().start();
        }

        /// Trigger a system reset.
        pub(crate) async fn trigger_reset(&self) -> ! {
            self.watchdog.lock().await.borrow_mut().trigger_reset()
        }

        /// Get the last reset reason.
        pub(crate) async fn reset_reason(&self) -> Option<ResetReason> {
            self.watchdog.lock().await.borrow().reset_reason()
        }

        /// Get the check interval
        pub(crate) async fn get_check_interval(&self) -> <EmbassyClock as Clock>::Duration {
            self.watchdog.lock().await.borrow().config.check_interval
        }

        /// Check if any tasks have starved
        pub(crate) async fn check_tasks(&self) -> bool {
            self.watchdog.lock().await.borrow_mut().check()
        }
    }

    /// A version of the Watchdog Task when not using the `alloc`` feature.
    ///
    /// There is an equivalent version of this when using the `alloc` feature
    /// which does not include the `const N: usize` type.
    pub struct WatchdogTask<const N: usize> {
        runner: &'static RpWatchdogOwner<N>,
    }

    impl<const N: usize> RpWatchdogOwner<N> {
        /// Used to create a watchdog task when not using the alloc feature.
        ///
        /// There is an equivalent version of this when using the `alloc` feature
        /// which does not include the `const N: usize` type.
        pub(crate) fn create_task(&'static self) -> WatchdogTask<N> {
            WatchdogTask { runner: self }
        }
    }

    /// Watchdog Runner function, which will monitor tasks and reset the
    /// system if any.
    ///
    /// You must call this function from an async task to start and run the
    /// watchdog.  Using `spawner.must_spawn(watchdog_run(watchdog))` would
    /// likely be a good choice.
    pub async fn watchdog_run<const N: usize>(task: WatchdogTask<N>) -> ! {
        info!("Watchdog runner started");

        // Start the watchdog
        task.runner.start().await;

        // Get initial check interval
        let interval = task.runner.get_check_interval().await;
        let mut check_time = Instant::now() + interval;

        loop {
            // Check for starved tasks.  We don't do anthing based on the
            // return code as check_tasks() handles feeding/starving the
            // hardware watchdog.
            let _ = task.runner.check_tasks().await;

            // Wait before checking again
            Timer::at(check_time).await;
            check_time += interval;
        }
    }

    use super::{BoundWatchdog, TaskDesc, TaskKey};

    // existing imports...
    // use embassy_rp::peripherals::WATCHDOG as RpWatchdogPeripheral;

    /// Auto-ID watchdog runner: fixes I = TaskKey.
    ///
    /// N defaults to 32 so the user doesn't have to write it.
    pub struct RpWatchdogRunner<const N: usize = 32> {
        inner: RpWatchdogOwner<N>,
    }

    impl<const N: usize> RpWatchdogRunner<N> {
        pub fn new(
            hw_watchdog: embassy_rp::Peri<'static, embassy_rp::peripherals::WATCHDOG>,
            config: WatchdogConfig<EmbassyClock>,
        ) -> Self {
            Self {
                inner: RpWatchdogOwner::new(hw_watchdog, config),
            }
        }

        #[inline(always)]
        pub async fn register_desc(
            &'static self,
            desc: &'static TaskDesc,
            max_duration: <EmbassyClock as Clock>::Duration,
        ) -> BoundWatchdog<'static, N> {
            let id = TaskKey::from_desc(desc);
            self.inner.register_task(&id, max_duration).await;
            BoundWatchdog::new(&self.inner, id)
        }

        // If you want to expose other runner methods, forward them:
        #[inline(always)]
        #[must_use]
        pub fn create_task(&'static self) -> WatchdogTask<N> {
            // use your existing create_task() on inner
            // (we need a &'static self; enforce via caller)
            // SAFETY: self is &'static in signature.
            let inner: &'static RpWatchdogOwner<N> = unsafe { &*(&self.inner as *const _) };
            inner.create_task()
        }
    }

    // Re-export for macro path convenience
    pub use RpWatchdogRunner as Watchdog;
}
