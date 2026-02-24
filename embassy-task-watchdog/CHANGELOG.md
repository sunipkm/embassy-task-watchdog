# Change Log for `embassy-task-watchdog`

### v0.0.5
- Removed the `watchdog_run` function in favor of `run` member function in the `WatchdogRunner` struct.
- Removed `embassy-task-watchdog::Error`, since it was never propagated and running out of task slots was relegated
  to a compile time error.
- Leverage the watchdog scratch registers in RP2040/RP235xy to indicate which task caused reset.
- Bumped `embassy-task-watchdog-macros` to `v0.0.3`, which brings extended functionality to the crate.
- Removed internal `TaskDesc` struct.
- Removed separate `create_watchdog_*` macros, and consolidated the functionality into a single `create_watchdog` macro.
- `create_watchdog` uses `static_cell::StaticCell` in its own context.
- Marking components activated by features in documentation.
- Added the ability to defer task stall by a set number of checks.

### v0.0.4

- Removed `usize` bounds since the maximum number of tasks is constrained by `MAX_TASKS`
  at compile time. This fixed a bug where the `usize` bounds were not set by default for `BoundWatchdog`, causing
  awkwardness in downstream programs.

### v0.0.3

- Fixed a bug where using the `create_watchdog!` macro would require an explicit import of embassy_time::Duration.
- Removed `create_watchdog!` in favor of `create_watchdog_rp!` and `create_watchdog_stm32!` to avoid requiring hidden imports.
- Updated documentation to bring more clarity to `BoundWatchdog`, `TaskWatchdog` and `WatchdogRunner`.
- Removed public export of `WatchdogSetup`, which is now handled by the `create_watchdog_*!` macros.
- Removed public export of `WatchdogConfig` struct fields.

### v0.0.2

- Exposed the `BoundWatchdog` type for the users to be able to inspect reset reason and trigger a reset from within a task.
- Feature `defmt` now enables `defmt` logging of various enums and structs.
- Feature `defmt-messages` logs various internal operations performed by the library.
- Updated `embassy-task-watchdog-macros` dependency to `v0.0.2`.
- Provide the `create_watchdog!` macro to simplify watchdog creation.
- Return `ResetReason` instead of `Option<ResetReason>` to minimize redirections.

### v0.0.1

- First release.
