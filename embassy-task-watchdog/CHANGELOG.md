# Change Log for `embassy-task-watchdog`
### v0.0.2
- Exposed the `BoundWatchdog` type for the users to be able to inspect reset reason and trigger a reset from within a task.
- Feature `defmt` now enables `defmt` logging of various enums and structs.
- Feature `defmt-messages` logs various internal operations performed by the library.
- Updated `embassy-task-watchdog-macros` dependency to `v0.0.2`.
- Provide the `create_watchdog!` macro to simplify watchdog creation.
- Return `ResetReason` instead of `Option<ResetReason>` to minimize redirections.

### v0.0.1
- First release.