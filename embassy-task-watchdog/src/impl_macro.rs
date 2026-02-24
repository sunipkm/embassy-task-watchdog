#[macro_export]
#[doc(hidden)]
/// Helper macro to implement the TaskWatchdog and WatchdogRunner for a family of watchdogs.
macro_rules! impl_watchdog {
    ($Family: ident) => {
        use paste::paste;
        paste!{
            use $crate::runtime::WatchdogOwner;
            /// The WatchdogRunner for this family of watchdogs.
            pub struct [<$Family WatchdogRunner>] {
                runner: &'static WatchdogOwner<[<$Family Watchdog>]>,
            }

            impl WatchdogOwner<[<$Family Watchdog>]> {
                /// Used to create a watchdog task when not using the alloc feature.
                ///
                /// There is an equivalent version of this when using the `alloc` feature
                /// which does not include the `const N: usize` type.
                pub(crate) fn create_task(&'static self) -> [<$Family WatchdogRunner>] {
                    [<$Family WatchdogRunner>] { runner: self }
                }
            }

            impl [<$Family WatchdogRunner>] {
                /// Watchdog Runner function, which will monitor tasks and reset the
                /// system if any.
                ///
                /// You must call this function in an [`embassy_executor::task`] to start and run the
                /// watchdog. Using [`embassy_executor::Spawner::must_spawn`] would likely be a good choice.
                pub async fn run(self) -> ! {
                    debug!("Watchdog runner started");

                    // Start the watchdog
                    self.runner.start().await;

                    // Get initial check interval
                    let interval = self.runner.get_check_interval().await;
                    let mut check_time = Instant::now() + interval;

                    loop {
                        // Check for starved tasks.  We don't do anthing based on the
                        // return code as check_tasks() handles feeding/starving the
                        // hardware watchdog.
                        let _ = self.runner.check_tasks().await;

                        // Wait before checking again
                        Timer::at(check_time).await;
                        check_time += interval;
                    }
                }
            }

            /// A per-task bound handle that is created from `TaskWatchdog` by the
            /// [`crate::task`] macro. This handle is re-bound with the same name
            /// as the original `TaskWatchdog` argument in the task function,
            /// and is used by the task to feed the watchdog, trigger a system reset,
            /// or get the reset reason. This struct can be passed to different
            /// functions called by the task.
            pub struct [<$Family BoundWatchdog>]<'a>
            where
                'a: 'static,
            {
                runner: &'a WatchdogOwner<[<$Family Watchdog>]>,
                id: u32,
            }

            impl<'a> [<$Family BoundWatchdog>]<'a> {
                #[inline(always)]
                pub(crate) fn new(runner: &'a WatchdogOwner<[<$Family Watchdog>]>, id: u32) -> Self {
                    Self { runner, id }
                }

                #[inline(always)]
                /// Feed the watchdog for this task.  This should be called periodically by the task to prevent the watchdog from resetting the system.
                /// Feeding the watchdog resets the `retries` count for the task.
                pub async fn feed(&self) {
                    self.runner.feed(self.id).await
                }

                #[inline(always)]
                #[doc(hidden)]
                /// Deregister this task from the watchdog.
                /// This is executed when the task exits, and is not intended to be called by user code.
                pub async fn _deregister(&self) {
                    self.runner.deregister_task(self.id).await
                }

                #[inline(always)]
                /// Get the reason for the last reset, if available.
                pub async fn reset_reason(&self) -> ResetReason {
                    self.runner.reset_reason().await
                }

                #[inline(always)]
                /// Trigger a reset immediately. This is useful for testing and for tasks that want to trigger a reset on their own.
                pub async fn trigger_reset(&self, reason: Option<heapless::String<32>>) -> ! {
                    self.runner.trigger_reset(self.id, reason).await
                }
            }

            /// The WatchdogSetup for this family of watchdogs.  This is the struct you create with `new()` and pass to the `build()` function to get the WatchdogRunner and TaskWatchdog.
            #[doc(hidden)]
            pub struct [<$Family WatchdogSetup>] {
                inner: WatchdogOwner<[<$Family Watchdog>]>,
            }

            impl [<$Family WatchdogSetup>] {
                #[inline(always)]
                #[must_use]
                /// Build the WatchdogRunner and TaskWatchdog for this family of watchdogs.
                pub fn build(&'static self) -> ([<$Family TaskWatchdog>], [<$Family WatchdogRunner>]) {
                    let iface = [<$Family TaskWatchdog>] { inner: &self.inner };
                    let task = self.create_task();
                    (iface, task)
                }

                // If you want to expose other runner methods, forward them:
                #[inline(always)]
                #[must_use]
                fn create_task(&'static self) -> [<$Family WatchdogRunner>] {
                    // use your existing create_task() on inner
                    // (we need a &'static self; enforce via caller)
                    // SAFETY: self is &'static in signature.
                    let inner: &'static WatchdogOwner<[<$Family Watchdog>]> =
                        unsafe { &*(&self.inner as *const _) };
                    inner.create_task()
                }
            }

            #[derive(Clone, Copy)]
            /// A per-task bound handle that is passed to the different tasks.
            /// The [`crate::task`] macro uses this struct, when it is provided
            /// by the user as the first argument, to register the task with
            /// the watchdog and then re-binds the argument to a `BoundWatchdog`
            /// for the task to feed the watchdog with.
            ///
            /// Pass this struct to the task, decorated by
            /// [`crate::task`] as the first argument.
            pub struct [<$Family TaskWatchdog>] {
                inner: &'static WatchdogOwner<[<$Family Watchdog>]>,
            }

            impl [<$Family TaskWatchdog>]{
                #[inline(always)]
                #[doc(hidden)]
                pub async fn _register_desc(
                    self,
                    name: &'static str,
                    id: u32,
                    max_duration: Duration,
                    retries: u8,
                ) -> [<$Family BoundWatchdog>]<'static> {
                    self.inner.register_task(id, name, max_duration, retries).await;
                    [<$Family BoundWatchdog>]::new(self.inner, id)
                }

                #[inline(always)]
                /// Get the reason for the last reset, if available.
                pub async fn reset_reason(&self) -> ResetReason {
                    self.inner.reset_reason().await
                }

                #[inline(always)]
                /// Trigger a watchdog reset.
                pub async fn trigger_reset(&self, reason: Option<heapless::String<32>>) -> ! {
                    self.inner.trigger_reset($crate::MAX_TASKS as u32, reason).await
                }
            }

            // Re-export for macro path convenience
            pub use [<$Family BoundWatchdog>] as BoundWatchdog;
            pub use [<$Family TaskWatchdog>] as TaskWatchdog;
            pub use [<$Family WatchdogSetup>] as Watchdog;
            pub use [<$Family WatchdogRunner>] as WatchdogRunner;

        }
    }
}
