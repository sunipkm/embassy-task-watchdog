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

// // pub(crate) type RpWatchdog = embassy_rp::Peri<'static, RpWatchdogPeripheral>;

// use crate::{MAX_TASKS, runtime::WatchdogOwner};

// /// A version of the Watchdog Task when not using the `alloc`` feature.
// ///
// /// There is an equivalent version of this when using the `alloc` feature
// /// which does not include the `const N: usize` type.
// pub struct RpWatchdogRunner<const N: usize = MAX_TASKS> {
//     runner: &'static WatchdogOwner<N, RpWatchdog>,
// }

// impl<const N: usize> WatchdogOwner<N, RpWatchdog> {
//     /// Used to create a watchdog task when not using the alloc feature.
//     ///
//     /// There is an equivalent version of this when using the `alloc` feature
//     /// which does not include the `const N: usize` type.
//     pub(crate) fn create_task(&'static self) -> RpWatchdogRunner<N> {
//         RpWatchdogRunner { runner: self }
//     }
// }

// /// Watchdog Runner function, which will monitor tasks and reset the
// /// system if any.
// ///
// /// You must call this function from an async task to start and run the
// /// watchdog.  Using `spawner.must_spawn(watchdog_run(watchdog))` would
// /// likely be a good choice.
// pub async fn watchdog_run<const N: usize>(task: RpWatchdogRunner<N>) -> ! {
//     info!("Watchdog runner started");

//     // Start the watchdog
//     task.runner.start().await;

//     // Get initial check interval
//     let interval = task.runner.get_check_interval().await;
//     let mut check_time = Instant::now() + interval;

//     loop {
//         // Check for starved tasks.  We don't do anthing based on the
//         // return code as check_tasks() handles feeding/starving the
//         // hardware watchdog.
//         let _ = task.runner.check_tasks().await;

//         // Wait before checking again
//         Timer::at(check_time).await;
//         check_time += interval;
//     }
// }

// use super::{TaskDesc, TaskKey};

// /// A per-task bound handle that lets the task call `feed()` without IDs.
// #[doc(hidden)]
// pub struct RpTaskWatchdogInner<'a, const N: usize>
// where
//     'a: 'static,
// {
//     runner: &'a WatchdogOwner<N, RpWatchdog>,
//     id: TaskKey,
// }

// impl<'a, const N: usize> RpTaskWatchdogInner<'a, N> {
//     #[inline(always)]
//     pub(crate) fn new(
//         runner: &'a WatchdogOwner<N, RpWatchdog>,
//         id: TaskKey,
//     ) -> Self {
//         Self { runner, id }
//     }

//     #[inline(always)]
//     pub async fn feed(&self) {
//         self.runner.feed(&self.id).await
//     }

//     #[inline(always)]
//     pub async fn deregister(&self) {
//         self.runner.deregister_task(&self.id).await
//     }

//     #[inline(always)]
//     pub async fn reset_reason(&self) -> Option<ResetReason> {
//         self.runner.reset_reason().await
//     }

//     #[inline(always)]
//     pub async fn trigger_reset(&self) -> ! {
//         self.runner.trigger_reset().await
//     }
// }

// // existing imports...
// // use embassy_rp::peripherals::WATCHDOG as RpWatchdogPeripheral;

// /// Auto-ID watchdog runner: fixes I = TaskKey.
// ///
// /// N defaults to 32 so the user doesn't have to write it.
// pub struct RpWatchdogSetup<const N: usize = MAX_TASKS> {
//     inner: WatchdogOwner<N, RpWatchdog>,
// }

// impl<const N: usize> RpWatchdogSetup<N> {
//     pub fn new(hw_watchdog: Peri<'static, RpWatchdogPeripheral>, config: WatchdogConfig) -> Self {
//         let hw_watchdog = RpWatchdog::new(hw_watchdog);
//         Self {
//             inner: WatchdogOwner::new(hw_watchdog, config),
//         }
//     }

//     // #[inline(always)]
//     // pub async fn register_desc(
//     //     &'static self,
//     //     desc: &'static TaskDesc,
//     //     max_duration: embassy_time::Duration,
//     // ) -> RpTaskWatchdog<'static, N> {
//     //     let id = TaskKey::from_desc(desc);
//     //     self.inner.register_task(&id, max_duration).await;
//     //     RpTaskWatchdog::new(&self.inner, id)
//     // }

//     #[inline(always)]
//     #[must_use]
//     pub fn build(&'static self) -> (RpTaskWatchdog<N>, RpWatchdogRunner<N>) {
//         let iface = RpTaskWatchdog { inner: &self.inner };
//         let task = self.create_task();
//         (iface, task)
//     }

//     // If you want to expose other runner methods, forward them:
//     #[inline(always)]
//     #[must_use]
//     fn create_task(&'static self) -> RpWatchdogRunner<N> {
//         // use your existing create_task() on inner
//         // (we need a &'static self; enforce via caller)
//         // SAFETY: self is &'static in signature.
//         let inner: &'static WatchdogOwner<N, RpWatchdog> = unsafe { &*(&self.inner as *const _) };
//         inner.create_task()
//     }
// }

// #[derive(Clone, Copy)]
// pub struct RpTaskWatchdog<const N: usize = MAX_TASKS> {
//     inner: &'static WatchdogOwner<N, RpWatchdog>,
// }

// impl<const N: usize> RpTaskWatchdog<N> {
//     #[inline(always)]
//     #[doc(hidden)]
//     pub async fn register_desc(
//         self,
//         desc: &'static TaskDesc,
//         max_duration: embassy_time::Duration,
//     ) -> RpTaskWatchdogInner<'static, N> {
//         let id = TaskKey::from_desc(desc);
//         self.inner.register_task(&id, max_duration).await;
//         RpTaskWatchdogInner::new(&self.inner, id)
//     }
// }

// // Re-export for macro path convenience
// pub use RpTaskWatchdog as TaskWatchdog;
// pub use RpWatchdogSetup as Watchdog;

crate::impl_watchdog!(Rp);

impl<const N: usize> RpWatchdogSetup<N> {
    /// Create a new RP2040/RP2350 watchdog setup.
    pub fn new(hw_watchdog: Peri<'static, RpWatchdogPeripheral>, config: WatchdogConfig) -> Self {
        let hw_watchdog = RpWatchdog::new(hw_watchdog);
        Self {
            inner: WatchdogOwner::new(hw_watchdog, config),
        }
    }
}