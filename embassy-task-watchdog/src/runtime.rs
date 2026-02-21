use crate::{Error, HardwareWatchdog, ResetReason, WatchdogConfig, debug, error, info, warn};

/// Descriptor emitted by the proc-macro (NOT in a linker section).
#[repr(C)]
#[doc(hidden)]
pub struct TaskDesc {
    pub name: &'static str,
}

/// Auto-generated ID type used internally by the auto-watchdog path.
///
/// We derive it from the address of a unique `static TaskDesc`.
/// It's Copy + Eq + Debug (and defmt::Format when enabled).
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[doc(hidden)]
pub struct TaskKey(u32);

impl core::fmt::Debug for TaskKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Show as hex; stable and useful
        write!(f, "TaskKey(0x{:08x})", self.0)
    }
}

impl TaskKey {
    /// Derive a stable-ish key for this boot from a descriptor's address.
    ///
    /// We hash the pointer down to u32 to keep the ID small.
    /// Collisions are extremely unlikely with a small number of tasks.
    #[inline(always)]
    #[doc(hidden)]
    pub fn from_desc(desc: &'static TaskDesc) -> Self {
        let addr = (desc as *const TaskDesc as usize) as u64;

        // A tiny fixed hash (no alloc, no std). Use splitmix64 then truncate.
        let mut x = addr.wrapping_add(0x9E3779B97F4A7C15);
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
        x ^= x >> 31;

        TaskKey((x as u32) ^ ((x >> 32) as u32))
    }
}

/// Represents a task monitored by the watchdog.
#[derive(Debug, Clone)]
pub(crate) struct Task {
    /// The task identifier.
    #[allow(dead_code)]
    id: TaskKey,

    /// The last time the task was fed.
    last_feed: embassy_time::Instant,

    /// Maximum duration between feeds.
    max_duration: embassy_time::Duration,
}

impl Task {
    /// Creates a new Task object for registration with the watchdog.
    pub fn new(id: TaskKey, max_duration: embassy_time::Duration) -> Self {
        Self {
            id,
            last_feed: embassy_time::Instant::now(),
            max_duration,
        }
    }

    /// Feed the task to indicate it's still active.
    fn feed(&mut self) {
        self.last_feed = embassy_time::Instant::now();
    }

    /// Check if this task has starved the watchdog.
    fn is_starved(&self) -> bool {
        embassy_time::Instant::now().duration_since(self.last_feed) > self.max_duration
    }
}

/// A Watchdog that monitors multiple tasks and feeds a hardware watchdog accordingly.
pub(crate) struct WatchdogContainer<const N: usize, W>
where
    W: HardwareWatchdog,
{
    /// The hardware watchdog.
    pub hw_watchdog: W,

    /// Tasks being monitored.
    pub tasks: [Option<Task>; N],

    /// Configuration.
    pub config: WatchdogConfig,
}

impl<W: HardwareWatchdog, const N: usize> WatchdogContainer<N, W> {
    /// Create a new watchdog with the given hardware watchdog and configuration.
    ///
    /// Arguments:
    /// * `hw_watchdog` - The hardware watchdog to use.
    /// * `config` - The configuration for the watchdog.
    /// * `clock` - The clock implementation to use for time-keeping.
    pub(crate) fn new(hw_watchdog: W, config: WatchdogConfig) -> Self {
        Self {
            hw_watchdog,
            tasks: [const { None }; N],
            config,
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
    pub(crate) fn register_task(
        &mut self,
        id: &TaskKey,
        max_duration: embassy_time::Duration,
    ) -> Result<(), Error> {
        // Find an empty slot
        for slot in &mut self.tasks {
            if slot.is_none() {
                *slot = Some(Task::new(*id, max_duration));
                debug!("Registered task: {:?}", id);
                return Ok(());
            }
        }

        // No empty slots available
        error!("Failed to register task: {:?} - no slots available", id);
        Err(Error::NoSlotsAvailable)
    }

    pub(crate) fn deregister_task(&mut self, id: &TaskKey) {
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

    pub(crate) fn feed(&mut self, id: &TaskKey) {
        let fed = self.tasks.iter_mut().flatten().any(|task| {
            if task.id == *id {
                task.feed();
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
    pub(crate) fn start(&mut self) {
        // Feed all registered tasks
        self.tasks.iter_mut().flatten().for_each(|task| {
            task.feed();
        });

        // Start the hardware watchdog
        self.hw_watchdog.start(self.config.hardware_timeout);

        info!("Watchdog started");
    }

    /// Check if any tasks have starved the watchdog and take appropriate action.
    pub(crate) fn check(&mut self) -> bool {
        // Check if any tasks have starved
        let mut starved = false;
        self.tasks.iter_mut().flatten().for_each(|task| {
            if task.is_starved() {
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
    pub(crate) fn trigger_reset(&mut self) -> ! {
        warn!("Triggering watchdog reset");
        self.hw_watchdog.trigger_reset()
    }

    /// Get the reason for the last reset.
    pub(crate) fn reset_reason(&self) -> Option<ResetReason> {
        self.hw_watchdog.reset_reason()
    }
}

pub(crate) struct WatchdogOwner<const N: usize, W: HardwareWatchdog> {
    watchdog: embassy_sync::mutex::Mutex<
        embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
        core::cell::RefCell<WatchdogContainer<N, W>>,
    >,
}

impl<const N: usize, W: HardwareWatchdog> WatchdogOwner<N, W> {
    /// Create a new Embassy-compatible watchdog runner.
    pub(crate) fn new(hw_watchdog: W, config: WatchdogConfig) -> Self {
        let watchdog = WatchdogContainer::new(hw_watchdog, config);
        Self {
            watchdog: embassy_sync::mutex::Mutex::new(core::cell::RefCell::new(watchdog)),
        }
    }

    /// Register a task with the watchdog.
    pub(crate) async fn register_task(&self, id: &TaskKey, max_duration: embassy_time::Duration) {
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
    pub(crate) async fn get_check_interval(&self) -> embassy_time::Duration {
        self.watchdog.lock().await.borrow().config.check_interval
    }

    /// Check if any tasks have starved
    pub(crate) async fn check_tasks(&self) -> bool {
        self.watchdog.lock().await.borrow_mut().check()
    }
}
