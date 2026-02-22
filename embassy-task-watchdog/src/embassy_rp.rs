use super::{HardwareWatchdog, ResetReason, WatchdogConfig, info};
use embassy_rp::watchdog::Watchdog as RpWatchdogDevice;
use embassy_rp::{Peri, peripherals::WATCHDOG as RpWatchdogPeripheral};
use embassy_time::{Instant, Timer};

/// RP2040/RP2350-specific watchdog implementation.
pub(crate) struct RpWatchdog {
    inner: RpWatchdogDevice,
}

impl RpWatchdog {
    /// Create a new RP2040/RP2350 watchdog.
    #[must_use]
    pub(crate) fn new(peripheral: Peri<'static, RpWatchdogPeripheral>) -> Self {
        Self {
            inner: RpWatchdogDevice::new(peripheral),
        }
    }
}

/// Implement the HardwareWatchdog trait for the RP2040/RP2350 watchdog.
impl HardwareWatchdog for RpWatchdog {
    fn start(&mut self, timeout: embassy_time::Duration) {
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

crate::impl_watchdog!(Rp);

impl<const N: usize> RpWatchdogSetup<N> {
    /// Create a new RP2040/RP2350 watchdog setup.
    pub fn new(
        hw_watchdog: Peri<'static, RpWatchdogPeripheral>,
        config: WatchdogConfig,
    ) -> Self {
        let hw_watchdog = RpWatchdog::new(hw_watchdog);
        Self {
            inner: WatchdogOwner::new(hw_watchdog, config),
        }
    }
}
