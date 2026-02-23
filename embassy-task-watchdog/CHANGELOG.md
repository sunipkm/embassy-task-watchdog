# Change Log for `embassy-task-watchdog`
### v0.0.3
- Fixed a bug where using the `create_watchdog!` macro would require an explicit import of embassy_time::Duration.
- Removed `create_watchdog!` in favor of `create_watchdog_rp!` and `create_watchdog_stm32!` to avoid requiring hidden imports.

### v0.0.2
- Exposed the `BoundWatchdog` type for the users to be able to inspect reset reason and trigger a reset from within a task.
- Feature `defmt` now enables `defmt` logging of various enums and structs.
- Feature `defmt-messages` logs various internal operations performed by the library.
- Updated `embassy-task-watchdog-macros` dependency to `v0.0.2`.
- Provide the `create_watchdog!` macro to simplify watchdog creation.
- Return `ResetReason` instead of `Option<ResetReason>` to minimize redirections.

### v0.0.1
- First release.