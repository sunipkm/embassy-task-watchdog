//! Number of tasks for the [`embassy_task_watchdog`] crate.
#![no_std]
mod config {
    #![allow(unused)]
    include!(concat!(env!("OUT_DIR"), "/config.rs"));
}
/// The maximum number of tasks that can be tracked by the [`embassy_task_watchdog`] crate.
pub use crate::config::MAX_TASKS;
