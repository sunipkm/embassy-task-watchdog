#![no_std]
#![no_main]
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::config::Config;
use embassy_task_watchdog::{
    WatchdogConfig, create_watchdog,
    embassy_rp::{RpWatchdogRunner, TaskWatchdog, Watchdog, watchdog_run},
};
use embassy_time::{Duration, Timer};
use panic_probe as _;
use static_cell::StaticCell;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialize the hardare peripherals
    let p = embassy_rp::init(Config::default());
    // Create the task watchdog and the watchdog runner.
    // Tasks feed the task watchdog to indicate life.
    // The watchdog runner feeds the hardware watchdog only if all tasks are alive.
    let (watchdog, watchdogtask) = create_watchdog!(p.WATCHDOG, WatchdogConfig::default());
    // Spawn tasks that will feed the watchdog
    spawner.must_spawn(main_task(watchdog));
    spawner.must_spawn(second_task(watchdog));
    // Finally spawn the watchdog - this will start the hardware watchdog, and feed it
    // for as long as _all_ tasks are healthy.
    spawner.must_spawn(watchdog_task(watchdogtask));
}
// Provide a simple embassy task for the watchdog
#[embassy_executor::task]
async fn watchdog_task(watchdog: RpWatchdogRunner) -> ! {
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
