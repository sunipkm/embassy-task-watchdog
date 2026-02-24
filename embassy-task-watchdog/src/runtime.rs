use core::cell::RefCell;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

use crate::{HardwareWatchdog, MAX_TASKS, ResetReason, WatchdogConfig, debug, error, info, warn};

/// Represents a task monitored by the watchdog.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) struct Task {
    /// The task name (for logging).
    #[allow(dead_code)]
    name: &'static str,

    /// The last time the task was fed.
    last_feed: embassy_time::Instant,

    /// Maximum duration between feeds.
    max_duration: embassy_time::Duration,

    /// Retries before the task is considered starved
    retries: u8,

    /// Available tries
    available: u8,
}

impl Task {
    /// Creates a new Task object for registration with the watchdog.
    pub fn new(name: &'static str, max_duration: embassy_time::Duration, retries: u8) -> Self {
        Self {
            name,
            last_feed: embassy_time::Instant::now(), // Initialize to the epoch; will be fed immediately on watchdog start.
            max_duration,
            retries,
            available: retries,
        }
    }

    /// Feed the task to indicate it's still active.
    fn feed(&mut self) {
        self.last_feed = embassy_time::Instant::now();
        self.available = self.retries;
    }

    /// Check if this task has starved the watchdog.
    fn is_starved(&mut self) -> bool {
        let res = embassy_time::Instant::now().duration_since(self.last_feed) > self.max_duration && self.available == 0;
        self.available = self.available.saturating_sub(1);
        res
    }
}

/// A Watchdog that monitors multiple tasks and feeds a hardware watchdog accordingly.
pub(crate) struct WatchdogContainer<W>
where
    W: HardwareWatchdog,
{
    /// The hardware watchdog.
    pub hw_watchdog: W,

    /// Tasks being monitored.
    pub tasks: [Option<Task>; MAX_TASKS],

    /// Configuration.
    pub config: WatchdogConfig,
}

impl<W: HardwareWatchdog> WatchdogContainer<W> {
    /// Create a new watchdog with the given hardware watchdog and configuration.
    ///
    /// Arguments:
    /// * `hw_watchdog` - The hardware watchdog to use.
    /// * `config` - The configuration for the watchdog.
    /// * `clock` - The clock implementation to use for time-keeping.
    pub(crate) fn new(hw_watchdog: W, config: WatchdogConfig) -> Self {
        Self {
            hw_watchdog,
            tasks: [const { None }; MAX_TASKS],
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
        id: u32,
        name: &'static str,
        max_duration: embassy_time::Duration,
        retries: u8,
    ) {
        // Find an empty slot
        if id >= MAX_TASKS as u32 {
            unreachable!("Task ID {} is out of bounds (max {})", id, MAX_TASKS - 1);
        }
        // SAFETY: We have already checked that the ID is within bounds, so this is safe.
        let slot = unsafe { self.tasks.get_unchecked_mut(id as usize) };
        *slot = Some(Task::new(name, max_duration, retries));
        debug!("Registered task: {} ({})", id, name);
    }

    pub(crate) fn deregister_task(&mut self, id: u32) {
        #[allow(unused)]
        if let Some(task) = self.tasks.get_mut(id as usize).and_then(|slot| slot.take()) {
            debug!("Deregistering task: {} ({})", id, task.name);
        } else {
            warn!("Attempted to deregister unknown task: {:?}", id);
        }
    }

    pub(crate) fn feed(&mut self, id: u32) {
        if let Some(task) = self
            .tasks
            .get_mut(id as usize)
            .and_then(|slot| slot.as_mut())
        {
            debug!("Feeding task: {} ({})", id, task.name);
            task.feed();
        } else {
            warn!("Attempted to feed unknown task: {:?}", id);
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
                error!("Task {} has starved the watchdog", task.name);
                starved = true;
            }
        });

        // Either feed the hardware watchdog or return that we have a starved
        // task
        if !starved {
            self.hw_watchdog.feed();
            debug!("Hardware watchdog fed");
        }

        starved
    }

    /// Trigger a system reset.
    pub(crate) fn trigger_reset(&mut self, id: u32, reason: Option<heapless::String<32>>) -> ! {
        #[allow(unused)]
        if let Some(task) = self
            .tasks
            .get_mut(id as usize)
            .and_then(|slot| slot.as_mut())
        {
            warn!("Task {} ({}) is triggering a watchdog reset", id, task.name);
        } else if id == MAX_TASKS as u32 {
            warn!(
                "Watchdog reset triggered by a TaskWatchdog (reason: {:?})",
                reason
            );
        } else {
            error!(
                "Unknown task {} attempted to trigger a watchdog reset, resetting",
                id
            );
        }
        self.hw_watchdog.trigger_reset(reason)
    }

    /// Get the reason for the last reset.
    pub(crate) fn reset_reason(&mut self) -> ResetReason {
        self.hw_watchdog.reset_reason()
    }
}

pub(crate) struct WatchdogOwner<W: HardwareWatchdog> {
    watchdog: Mutex<CriticalSectionRawMutex, RefCell<WatchdogContainer<W>>>,
}

impl<W: HardwareWatchdog> WatchdogOwner<W> {
    /// Create a new Embassy-compatible watchdog runner.
    pub(crate) fn new(hw_watchdog: W, config: WatchdogConfig) -> Self {
        let watchdog = WatchdogContainer::new(hw_watchdog, config);
        Self {
            watchdog: Mutex::new(RefCell::new(watchdog)),
        }
    }

    /// Register a task with the watchdog.
    pub(crate) async fn register_task(
        &self,
        id: u32,
        name: &'static str,
        max_duration: embassy_time::Duration,
        retries: u8,
    ) {
        self.watchdog
            .lock()
            .await
            .borrow_mut()
            .register_task(id, name, max_duration, retries);
    }

    /// Deregister a task with the watchdog.
    pub(crate) async fn deregister_task(&self, id: u32) {
        self.watchdog.lock().await.borrow_mut().deregister_task(id);
    }

    /// Feed the watchdog for a specific task.
    pub(crate) async fn feed(&self, id: u32) {
        debug!("Feeding watchdog for task ID {}", id);
        self.watchdog.lock().await.borrow_mut().feed(id);
    }

    /// Start the watchdog.
    pub(crate) async fn start(&self) {
        self.watchdog.lock().await.borrow_mut().start();
    }

    /// Trigger a system reset.
    pub(crate) async fn trigger_reset(&self, id: u32, reason: Option<heapless::String<32>>) -> ! {
        self.watchdog
            .lock()
            .await
            .borrow_mut()
            .trigger_reset(id, reason)
    }

    /// Get the last reset reason.
    pub(crate) async fn reset_reason(&self) -> ResetReason {
        self.watchdog.lock().await.borrow_mut().reset_reason()
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
