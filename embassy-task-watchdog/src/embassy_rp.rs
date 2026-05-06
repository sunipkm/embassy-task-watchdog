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
    timeout: Duration,
}

impl RpWatchdog {
    /// Create a new RP2040/RP2350 watchdog.
    #[must_use]
    pub(crate) fn new(peripheral: Peri<'static, RpWatchdogPeripheral>) -> Self {
        Self {
            inner: RpWatchdogDevice::new(peripheral),
            timeout: Duration::from_micros(1_000_000), // Default timeout of 1 second
        }
    }

    #[inline(always)]
    pub(crate) fn write_reason_str(&mut self, reason: Option<String<32>>) {
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
    }

    fn read_reason_str(&mut self) -> String<32> {
        let mut reason_bytes = Vec::<u8, 32>::new();
        for idx in 0..8 {
            let value = self.inner.get_scratch(idx);
            // SAFETY: There are 8, u32 scratch registers, so we can safely read them into a 32-byte buffer.
            reason_bytes
                .extend_from_slice(&value.to_le_bytes())
                .unwrap();
        }
        String::from_utf8(reason_bytes).unwrap_or_else(|_| String::from_str("Unknown").unwrap())
    }
}

/// Implement the HardwareWatchdog trait for the RP2040/RP2350 watchdog.
impl HardwareWatchdog for RpWatchdog {
    fn start(&mut self, timeout: Duration) {
        self.timeout = timeout;
        self.inner.start(timeout);
    }

    fn feed(&mut self) {
        self.inner.feed(self.timeout);
    }

    fn trigger_reset(&mut self, reason: Option<String<32>>) -> ! {
        self.write_reason_str(reason);
        // Trigger system reset
        self.inner.trigger_reset();
        panic!("Triggering reset via watchdog failed");
    }

    fn reset_reason(&mut self) -> ResetReason {
        self.inner
            .reset_reason()
            .map(|reason| match reason {
                embassy_rp::watchdog::ResetReason::Forced => {
                    ResetReason::Forced(self.read_reason_str())
                }
                embassy_rp::watchdog::ResetReason::TimedOut => {
                    ResetReason::TimedOut(self.read_reason_str())
                }
            })
            .unwrap_or(ResetReason::None)
    }

    fn _write_reason(&mut self, reason: Option<heapless::String<32>>) {
        self.write_reason_str(reason);
    }

    #[inline(always)]
    fn _reason_supported() -> bool {
        true
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
