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

impl Stm32WatchdogSetup {
    /// Create a new STM32 watchdog setup.
    pub fn new(hw_watchdog: Peri<'static, IWDG>, config: WatchdogConfig) -> Self {
        let hw_watchdog = Stm32Watchdog::new(hw_watchdog, config.check_interval.as_micros() as u32);
        Self {
            inner: WatchdogOwner::new(hw_watchdog, config),
        }
    }
}
