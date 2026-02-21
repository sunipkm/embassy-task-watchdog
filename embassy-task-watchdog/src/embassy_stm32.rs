use super::{HardwareWatchdog, ResetReason, WatchdogConfig, info};
use embassy_stm32::wdg::IndependentWatchdog;
use embassy_stm32::{Peri, peripherals::IWDG};
use embassy_time::{Instant, Timer};

pub(crate) struct Stm32Watchdog {
    inner: IndependentWatchdog<'static, IWDG>,
}

impl Stm32Watchdog {
    /// Create a new STM32 watchdog.
    #[must_use]
    pub(crate) fn new(peripheral: embassy_stm32::Peri<'static, IWDG>, timeout_us: u32) -> Self {
        Self {
            inner: IndependentWatchdog::new(peripheral, timeout_us), // Default timeout of 1 second
        }
    }
}

impl HardwareWatchdog for Stm32Watchdog {
    fn start(&mut self, timeout: embassy_time::Duration) {
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

    fn reset_reason(&self) -> Option<ResetReason> {
        None
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