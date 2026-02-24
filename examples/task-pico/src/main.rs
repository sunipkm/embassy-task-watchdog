#![no_std]
#![no_main]
use core::str::FromStr as _;

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::config::Config;
use embassy_task_watchdog::{
    WatchdogConfig, create_watchdog,
    embassy_rp::{TaskWatchdog, WatchdogRunner},
};
use embassy_time::{Duration, Timer};
use panic_probe as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialize the hardare peripherals
    let p = embassy_rp::init(Config::default());
    // Create the task watchdog and the watchdog runner.
    // Tasks feed the task watchdog to indicate life.
    // The watchdog runner feeds the hardware watchdog only if all tasks are alive.
    let (watchdog, watchdogtask) = create_watchdog!(p.WATCHDOG, WatchdogConfig::default());
    // Check the last reset reason and print it
    info!("Last reset reason: {}", watchdog.reset_reason().await);
    // Spawn tasks that will feed the watchdog
    spawner.must_spawn(main_task(watchdog));
    spawner.must_spawn(second_task(watchdog));
    // Finally spawn the watchdog - this will start the hardware watchdog, and feed it
    // for as long as _all_ tasks are healthy.
    spawner.must_spawn(watchdog_task(watchdogtask));
}

// Provide a simple embassy task for the watchdog
#[embassy_executor::task]
async fn watchdog_task(watchdog: WatchdogRunner) -> ! {
    watchdog.run().await
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

// Implement your second task, which requires a long setup time
#[embassy_task_watchdog::task(timeout = Duration::from_millis(2000), setup = true)]
async fn second_task(watchdog: TaskWatchdog) -> ! {
    info!("Starting second task, doing setup work...");
    // do some long running setup work
    let mut counter = 0;
    Timer::after_secs(5).await;
    // We can have another loop here, but
    // it is part of setup code and not the
    // main loop of the task, so the task is
    // not registered yet
    for _ in 0..5 {
        // Do some setup work
        Timer::after(Duration::from_millis(1000)).await;
    }
    info!("Finished setup work, entering main loop");
    // Task registration happens here
    // Now enter the main loop of the task
    loop {
        // Feed the watchdog
        if counter < 5 {
            watchdog.feed().await;
            counter += 1;
        } else {
            info!("Resetting the watchdog");
            // Trigger a reset after 5 iterations to demonstrate the reset reason functionality.
            // Reset reason is a 32-byte string.
            watchdog
                .trigger_reset(Some(heapless::String::from_str("MockReset").unwrap()))
                .await;
        }
        // Do some work
        Timer::after(Duration::from_millis(2000)).await;
    }
}
