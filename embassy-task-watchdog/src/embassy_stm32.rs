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

// use crate::{MAX_TASKS, runtime::WatchdogOwner};

// pub struct Stm32WatchdogRunner<const N: usize = MAX_TASKS> {
//     runner: &'static WatchdogOwner<N, Stm32Watchdog>,
// }

// impl<const N: usize> WatchdogOwner<N, Stm32Watchdog> {
//     /// Used to create a watchdog task when not using the alloc feature.
//     ///
//     /// There is an equivalent version of this when using the `alloc` feature
//     /// which does not include the `const N: usize` type.
//     pub(crate) fn create_task(&'static self) -> Stm32WatchdogRunner<N> {
//         Stm32WatchdogRunner { runner: self }
//     }
// }

// /// Watchdog Runner function, which will monitor tasks and reset the
// /// system if any.
// ///
// /// You must call this function from an async task to start and run the
// /// watchdog.  Using `spawner.must_spawn(watchdog_run(watchdog))` would
// /// likely be a good choice.
// pub async fn watchdog_run<const N: usize>(task: Stm32WatchdogRunner<N>) -> ! {
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
// pub struct Stm32TaskWatchdogInner<'a, const N: usize>
// where
//     'a: 'static,
// {
//     runner: &'a WatchdogOwner<N, Stm32Watchdog>,
//     id: TaskKey,
// }

// impl<'a, const N: usize> Stm32TaskWatchdogInner<'a, N> {
//     #[inline(always)]
//     pub(crate) fn new(runner: &'a WatchdogOwner<N, Stm32Watchdog>, id: TaskKey) -> Self {
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
// pub struct Stm32WatchdogSetup<const N: usize = MAX_TASKS> {
//     inner: WatchdogOwner<N, Stm32Watchdog>,
// }

// impl<const N: usize> Stm32WatchdogSetup<N> {
//     pub fn new(hw_watchdog: Peri<'static, IWDG>, config: WatchdogConfig) -> Self {
//         let hw_watchdog = Stm32Watchdog::new(hw_watchdog, config.check_interval.as_micros() as u32);
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
//     pub fn build(&'static self) -> (Stm32TaskWatchdog<N>, Stm32WatchdogRunner<N>) {
//         let iface = Stm32TaskWatchdog { inner: &self.inner };
//         let task = self.create_task();
//         (iface, task)
//     }

//     // If you want to expose other runner methods, forward them:
//     #[inline(always)]
//     #[must_use]
//     fn create_task(&'static self) -> Stm32WatchdogRunner<N> {
//         // use your existing create_task() on inner
//         // (we need a &'static self; enforce via caller)
//         // SAFETY: self is &'static in signature.
//         let inner: &'static WatchdogOwner<N, Stm32Watchdog> =
//             unsafe { &*(&self.inner as *const _) };
//         inner.create_task()
//     }
// }

// #[derive(Clone, Copy)]
// pub struct Stm32TaskWatchdog<const N: usize = MAX_TASKS> {
//     inner: &'static WatchdogOwner<N, Stm32Watchdog>,
// }

// impl<const N: usize> Stm32TaskWatchdog<N> {
//     #[inline(always)]
//     #[doc(hidden)]
//     pub async fn register_desc(
//         self,
//         desc: &'static TaskDesc,
//         max_duration: embassy_time::Duration,
//     ) -> Stm32TaskWatchdogInner<'static, N> {
//         let id = TaskKey::from_desc(desc);
//         self.inner.register_task(&id, max_duration).await;
//         Stm32TaskWatchdogInner::new(self.inner, id)
//     }
// }

// // Re-export for macro path convenience
// pub use Stm32TaskWatchdog as TaskWatchdog;
// pub use Stm32WatchdogSetup as Watchdog;

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