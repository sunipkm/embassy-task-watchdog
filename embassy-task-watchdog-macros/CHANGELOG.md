# Change Log for `embassy-task-watchdog-macros`

### v0.0.3

- Added additional options (`setup`, `keep`, `fallible`) to the task macro:
  - `setup` allows breaking the task code into a main setup body, during which
    the task is not registered into the watchdog registry to allow for possibly
    long-running setup tasks to complete. Such a function body must start with
    expressions, and contain a `loop {...}` after the expressions. All expressions
    before this `loop {...}` is considered setup code, and the task registration
    happens after the last expression code and before the `loop {...}` begins
    execution.
  - Setting `keep` to `false` allows a task to deregister itself from the
    watchdog. It is recommended to set `fallible = true` in this scenario.
  - Setting `fallible` to `true` relaxes the requirement that the tasks never
    return (`-> !`). It is recommended to be used in conjunction with `keep`.
- Remove forced task deregistration at the end of the user task function at the
  end of the macro expansion.

### v0.0.2

- Disabled the maximum number of tasks check for debug compilations.
- Updated documentation.

### v0.0.1

- First release.
