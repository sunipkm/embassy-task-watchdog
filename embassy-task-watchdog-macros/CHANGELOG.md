# Change Log for `embassy-task-watchdog-macros`
### v0.0.3
- Require tasks annotated with the `task` macro to never return (`async fn func(wdt) -> !`).
- Remove task deregistration at the end of the user task function at the end of the macro expansion.

### v0.0.2
- Disabled the maximum number of tasks check for debug compilations.
- Updated documentation.

### v0.0.1
- First release.