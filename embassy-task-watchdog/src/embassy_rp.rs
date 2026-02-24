use core::str::FromStr;

use super::{HardwareWatchdog, ResetReason, WatchdogConfig, debug};
use embassy_rp::{
    Peri, peripherals::WATCHDOG as RpWatchdogPeripheral, watchdog::Watchdog as RpWatchdogDevice,
};
use embassy_time::{Duration, Instant, Timer};
use heapless::{String, Vec};

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
    fn start(&mut self, timeout: Duration) {
        self.inner.start(timeout);
    }

    fn feed(&mut self) {
        self.inner.feed();
    }

    fn trigger_reset(&mut self, reason: Option<String<32>>) -> ! {
        let reason = reason.unwrap_or_else(|| String::from_str("Unknown").unwrap());
        debug!("Triggering reset with reason: {}", reason);
        // Zero out the scratch registers
        for idx in 0..8 {
            self.inner.set_scratch(idx, 0);
        }
        // Write the reason string into the scratch registers, 4 bytes at a time.
        for (idx, chunk) in reason.as_bytes().chunks(4).enumerate() {
            let mut scratch = [0u8; 4];
            scratch[..chunk.len()].copy_from_slice(chunk);
            let value = u32::from_le_bytes(scratch);
            self.inner.set_scratch(idx, value);
        }
        // Trigger system reset
        self.inner.trigger_reset();
        panic!("Triggering reset via watchdog failed");
    }

    fn reset_reason(&mut self) -> ResetReason {
        self.inner
            .reset_reason()
            .map(|reason| match reason {
                embassy_rp::watchdog::ResetReason::Forced => {
                    let mut msg = Vec::new();
                    for index in 0..8 {
                        let scratch = self.inner.get_scratch(index);
                        // SAFETY: There are 8, u32 scratch registers, so we can safely read them into a 32-byte buffer.
                        msg.extend_from_slice(&scratch.to_le_bytes()).unwrap();
                    }
                    ResetReason::Forced(
                        String::from_utf8(msg).unwrap_or(String::from_str("Unknown").unwrap()),
                    )
                }
                embassy_rp::watchdog::ResetReason::TimedOut => ResetReason::TimedOut,
            })
            .unwrap_or(ResetReason::None)
    }
}

crate::impl_watchdog!(Rp);

impl RpWatchdogSetup {
    /// Create a new RP2040/RP2350 watchdog setup.
    pub fn new(hw_watchdog: Peri<'static, RpWatchdogPeripheral>, config: WatchdogConfig) -> Self {
        let hw_watchdog = RpWatchdog::new(hw_watchdog);
        Self {
            inner: WatchdogOwner::new(hw_watchdog, config),
        }
    }
}
