use super::{HardwareWatchdog, ResetReason, WatchdogConfig, debug};
use embassy_stm32::{Peri, peripherals::IWDG, wdg::IndependentWatchdog};
use embassy_time::{Duration, Instant, Timer};

pub(crate) struct Stm32Watchdog {
    inner: IndependentWatchdog<'static, IWDG>,
}

impl Stm32Watchdog {
    /// Create a new STM32 watchdog.
    #[must_use]
    pub(crate) fn new(peripheral: Peri<'static, IWDG>, timeout_us: u32) -> Self {
        Self {
            inner: IndependentWatchdog::new(peripheral, timeout_us), // Default timeout of 1 second
        }
    }
}

impl HardwareWatchdog for Stm32Watchdog {
    fn start(&mut self, timeout: Duration) {
        let timeout = timeout.as_micros();
        if timeout > u32::MAX as u64 {
            panic!("Watchdog timeout too large for STM32");
        }

        // Start it
        self.inner.unleash();
    }

    fn feed(&mut self) {
        self.inner.pet();
    }

    fn trigger_reset(&mut self) -> ! {
        cortex_m::peripheral::SCB::sys_reset();
    }

    fn reset_reason(&self) -> ResetReason {
        ResetReason::Unknown
    }
}

crate::impl_watchdog!(Stm32);

impl<const N: usize> Stm32WatchdogSetup<N> {
    /// Create a new STM32 watchdog setup.
    pub fn new(hw_watchdog: Peri<'static, IWDG>, config: WatchdogConfig) -> Self {
        let hw_watchdog = Stm32Watchdog::new(hw_watchdog, config.check_interval.as_micros() as u32);
        Self {
            inner: WatchdogOwner::new(hw_watchdog, config),
        }
    }
}

/// Initialize the static memory for the watchdog, and return the watchdog and
/// the watchdog runner task. Pass the [`TaskWatchdog` struct](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_rp/struct.RpTaskWatchdog.html) to your tasks to be able to feed the watchdog. Pass the [`WatchdogRunner` struct](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_rp/struct.RpWatchdogRunner.html) to the [`watchdog_run` function](https://docs.rs/embassy-task-watchdog/latest/embassy_task_watchdog/embassy_rp/fn.watchdog_run.html) inside a spawned task to monitor the tasks and feed the hardware watchdog.
/// ```rust
/// # #![no_std]
/// # #![no_main]
/// # use defmt_rtt as _;
/// # use embassy_executor::Spawner;
/// # use embassy_rp::config::Config;
/// # use embassy_task_watchdog::{
/// #     WatchdogConfig, create_watchdog_stm32,
/// #     embassy_stm32::{TaskWatchdog, WatchdogRunner, watchdog_run},
/// # };
/// # use embassy_time::{Duration, Timer};
/// # use panic_probe as _;
/// # use static_cell::StaticCell;
/// # use embassy_time::{Duration, Timer};
/// # use panic_probe as _;
/// # use static_cell::StaticCell;
///
/// #[embassy_executor::main]
/// async fn main(spawner: Spawner) {
///     // Initialize the hardare peripherals
///     let p = embassy_rp::init(Config::default());
///     // Create the watchdog runner, store it in a static cell, and get the watchdog and watchdog runner task.
///     let (watchdog, watchdogtask) = create_watchdog_stm32!(p.IWDG, config);
///     // Spawn tasks that will feed the watchdog
///     spawner.must_spawn(main_task(watchdog));
///     spawner.must_spawn(second_task(watchdog));
///     // Finally spawn the watchdog - this will start the hardware watchdog, and feed it
///     // for as long as _all_ tasks are healthy.
///     spawner.must_spawn(watchdog_task(watchdogtask));
/// }
/// // Provide a simple embassy task for the watchdog
/// #[embassy_executor::task]
/// async fn watchdog_task(watchdog: WatchdogRunner) -> ! {
///     watchdog_run(watchdog).await
/// }
/// // Implement your main task
/// #[embassy_task_watchdog::task(timeout = Duration::from_millis(1500))]
/// async fn main_task(watchdog: TaskWatchdog) -> ! {
///     loop {
///         // Feed the watchdog
///         watchdog.feed().await;
///         // Do some work
///         Timer::after(Duration::from_millis(1000)).await;
///     }
/// }
/// // Implement your second task
/// #[embassy_task_watchdog::task(timeout = Duration::from_millis(2000))]
/// async fn second_task(watchdog: TaskWatchdog) -> ! {
///     loop {
///         // Feed the watchdog
///         watchdog.feed().await;
///         // Do some work
///         Timer::after(Duration::from_millis(2000)).await;
///     }
/// }
/// ```
#[macro_export]
macro_rules! create_watchdog_stm32 {
    ($wdt: expr, $config: expr) => {{
        use $crate::embassy_stm32::Watchdog;
        // Create a static to hold the task-watchdog object, so it has static
        // lifetime and can be shared with tasks.
        static WATCHDOG: StaticCell<Watchdog> = StaticCell::new();
        // Create the watchdog runner and store it in the static cell
        let watchdog = Watchdog::new($wdt, $config);
        WATCHDOG.init(watchdog).build()
    }};
}