#![no_std]
#![no_main]

// use defmt::*;
use embassy_executor::Spawner;
use embassy_task_watchdog::WatchdogConfig;
use embassy_task_watchdog::embassy_stm32::{
    Stm32WatchdogRunner, TaskWatchdog, Watchdog, watchdog_run,
};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialize the embassy runtime - this will set up the system clock, and the timer driver that embassy uses for async timing.
    let p = embassy_stm32::init(Default::default());
    // Create a static to hold the task-watchdog object, so it has static
    // lifetime and can be shared with tasks.
    static WATCHDOG: StaticCell<Watchdog> = StaticCell::new();
    // Set up watchdog configuration, with a 5s hardware watchdog timeout, and
    // with the task watchdog checking tasks every second.
    let config = WatchdogConfig::default();
    // Create the hardware watchdog object, which will be used to feed the hardware watchdog, and to create the task watchdog runner.
    let watchdog = Watchdog::new(p.IWDG, config);
    // Initialize and build the watchdog, which will return a runner that can be used to feed the hardware watchdog, and a task that will run the task watchdog.
    let (watchdog, watchdogtask) = WATCHDOG.init(watchdog).build();
    // Register our tasks with the task-watchdog.  Each can have a different timeout.
    // Spawn tasks that will feed the watchdog
    spawner.must_spawn(main_task(watchdog));
    spawner.must_spawn(second_task(watchdog));
    // Finally spawn the watchdog - this will start the hardware watchdog, and feed it
    // for as long as _all_ tasks are healthy.
    spawner.must_spawn(watchdog_task(watchdogtask));
}
// Provide a simple embassy task for the watchdog
#[embassy_executor::task]
async fn watchdog_task(watchdog: Stm32WatchdogRunner) -> ! {
    watchdog_run(watchdog).await
}
// Implement your main task
#[embassy_task_watchdog::task(timeout = Duration::from_millis(1500))]
async fn main_task(watchdog: TaskWatchdog) -> ! {
    loop {
        // Feed the watchdog
        watchdog.feed().await;
        // Do some work
        Timer::after(Duration::from_millis(1000)).await;
    }
}
// Implement your second task
#[embassy_task_watchdog::task(timeout = Duration::from_millis(2000))]
async fn second_task(watchdog: TaskWatchdog) -> ! {
    loop {
        // Feed the watchdog
        watchdog.feed().await;
        // Do some work
        Timer::after(Duration::from_millis(2000)).await;
    }
}
