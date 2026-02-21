use super::{HardwareWatchdog, ResetReason, WatchdogConfig, info, runtime::WatchdogContainer};
use embassy_rp::peripherals::WATCHDOG as RpWatchdogPeripheral;
use embassy_rp::watchdog::Watchdog as RpWatchdogDevice;
use embassy_time::{Instant, Timer};

/// RP2040/RP2350-specific watchdog implementation.
struct RpWatchdog {
    inner: RpWatchdogDevice,
}

impl RpWatchdog {
    /// Create a new RP2040/RP2350 watchdog.
    #[must_use]
    pub(crate) fn new(peripheral: embassy_rp::Peri<'static, RpWatchdogPeripheral>) -> Self {
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

// pub(crate) type RpWatchdog = embassy_rp::Peri<'static, RpWatchdogPeripheral>;

use crate::runtime::WatchdogOwner as RpWatchdogOwner;

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
// pub(crate) struct RpWatchdogOwner<const N: usize> {
//     watchdog: embassy_sync::mutex::Mutex<
//         embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
//         core::cell::RefCell<WatchdogContainer<N, RpWatchdog, EmbassyClock>>,
//     >,
// }

// impl<const N: usize> RpWatchdogOwner<N> {
//     /// Create a new Embassy-compatible watchdog runner.
//     pub(crate) fn new(
//         hw_watchdog: embassy_rp::Peri<'static, RpWatchdogPeripheral>,
//         config: WatchdogConfig<EmbassyClock>,
//     ) -> Self {
//         let hw_watchdog = RpWatchdog::new(hw_watchdog);
//         let watchdog = WatchdogContainer::new(hw_watchdog, config, EmbassyClock);
//         Self {
//             watchdog: embassy_sync::mutex::Mutex::new(core::cell::RefCell::new(watchdog)),
//         }
//     }

//     /// Register a task with the watchdog.
//     pub(crate) async fn register_task(
//         &self,
//         id: &TaskKey,
//         max_duration: <EmbassyClock as Clock>::Duration,
//     ) {
//         self.watchdog
//             .lock()
//             .await
//             .borrow_mut()
//             .register_task(id, max_duration)
//             .ok();
//     }

//     /// Deregister a task with the watchdog.
//     pub(crate) async fn deregister_task(&self, id: &TaskKey) {
//         self.watchdog.lock().await.borrow_mut().deregister_task(id);
//     }

//     /// Feed the watchdog for a specific task.
//     pub(crate) async fn feed(&self, id: &TaskKey) {
//         self.watchdog.lock().await.borrow_mut().feed(id);
//     }

//     /// Start the watchdog.
//     pub(crate) async fn start(&self) {
//         self.watchdog.lock().await.borrow_mut().start();
//     }

//     /// Trigger a system reset.
//     pub(crate) async fn trigger_reset(&self) -> ! {
//         self.watchdog.lock().await.borrow_mut().trigger_reset()
//     }

//     /// Get the last reset reason.
//     pub(crate) async fn reset_reason(&self) -> Option<ResetReason> {
//         self.watchdog.lock().await.borrow().reset_reason()
//     }

//     /// Get the check interval
//     pub(crate) async fn get_check_interval(&self) -> <EmbassyClock as Clock>::Duration {
//         self.watchdog.lock().await.borrow().config.check_interval
//     }

//     /// Check if any tasks have starved
//     pub(crate) async fn check_tasks(&self) -> bool {
//         self.watchdog.lock().await.borrow_mut().check()
//     }
// }

/// A version of the Watchdog Task when not using the `alloc`` feature.
///
/// There is an equivalent version of this when using the `alloc` feature
/// which does not include the `const N: usize` type.
pub struct WatchdogTask<const N: usize = 32> {
    runner: &'static RpWatchdogOwner<N, RpWatchdog>,
}

impl<const N: usize> RpWatchdogOwner<N, RpWatchdog> {
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

use super::{TaskDesc, TaskKey};

/// A per-task bound handle that lets the task call `feed()` without IDs.
pub struct RpTaskWatchdog<'a, const N: usize>
where
    'a: 'static,
{
    runner: &'a crate::embassy_rp::RpWatchdogOwner<N, RpWatchdog>,
    id: TaskKey,
}

impl<'a, const N: usize> RpTaskWatchdog<'a, N> {
    #[inline(always)]
    pub(crate) fn new(
        runner: &'a crate::embassy_rp::RpWatchdogOwner<N, RpWatchdog>,
        id: TaskKey,
    ) -> Self {
        Self { runner, id }
    }

    #[inline(always)]
    pub async fn feed(&self) {
        self.runner.feed(&self.id).await
    }

    #[inline(always)]
    pub async fn deregister(&self) {
        self.runner.deregister_task(&self.id).await
    }

    #[inline(always)]
    pub(crate) fn id(&self) -> TaskKey {
        self.id
    }
}

// existing imports...
// use embassy_rp::peripherals::WATCHDOG as RpWatchdogPeripheral;

/// Auto-ID watchdog runner: fixes I = TaskKey.
///
/// N defaults to 32 so the user doesn't have to write it.
pub struct RpWatchdogRunner<const N: usize = 32> {
    inner: RpWatchdogOwner<N, RpWatchdog>,
}

impl<const N: usize> RpWatchdogRunner<N> {
    pub fn new(
        hw_watchdog: embassy_rp::Peri<'static, embassy_rp::peripherals::WATCHDOG>,
        config: WatchdogConfig,
    ) -> Self {
        let hw_watchdog = RpWatchdog::new(hw_watchdog);
        Self {
            inner: RpWatchdogOwner::new(hw_watchdog, config),
        }
    }

    // #[inline(always)]
    // pub async fn register_desc(
    //     &'static self,
    //     desc: &'static TaskDesc,
    //     max_duration: embassy_time::Duration,
    // ) -> RpTaskWatchdog<'static, N> {
    //     let id = TaskKey::from_desc(desc);
    //     self.inner.register_task(&id, max_duration).await;
    //     RpTaskWatchdog::new(&self.inner, id)
    // }

    #[inline(always)]
    #[must_use]
    pub fn build(&'static self) -> (RpWatchdogIface<N>, WatchdogTask<N>) {
        let iface = RpWatchdogIface { inner: &self.inner };
        let task = self.create_task();
        (iface, task)
    }

    // If you want to expose other runner methods, forward them:
    #[inline(always)]
    #[must_use]
    fn create_task(&'static self) -> WatchdogTask<N> {
        // use your existing create_task() on inner
        // (we need a &'static self; enforce via caller)
        // SAFETY: self is &'static in signature.
        let inner: &'static RpWatchdogOwner<N, RpWatchdog> = unsafe { &*(&self.inner as *const _) };
        inner.create_task()
    }
}

#[derive(Clone, Copy)]
pub struct RpWatchdogIface<const N: usize = 32> {
    inner: &'static RpWatchdogOwner<N, RpWatchdog>,
}

impl<const N: usize> RpWatchdogIface<N> {
    #[inline(always)]
    pub async fn register_desc(
        self,
        desc: &'static TaskDesc,
        max_duration: embassy_time::Duration,
    ) -> RpTaskWatchdog<'static, N> {
        let id = TaskKey::from_desc(desc);
        self.inner.register_task(&id, max_duration).await;
        RpTaskWatchdog::new(&self.inner, id)
    }
}

// Re-export for macro path convenience
pub use RpWatchdogRunner as Watchdog;
pub use RpWatchdogIface as WatchdogIface;
