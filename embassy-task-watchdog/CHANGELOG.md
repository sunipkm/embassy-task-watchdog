# Change Log for `embassy-task-watchdog`

### v0.0.5

- Bumped `embassy-task-watchdog-macros` to `v0.0.3`.
- Reduced memory usage of internal `TaskDesc` static variable.
- Removed separate `create_watchdog_*` macros, and consolidated the functionality into a single `create_watchdog` macro.
- `create_watchdog` uses `static_cell::StaticCell` in its own context.
- Marking components activated by features in documentation.

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
