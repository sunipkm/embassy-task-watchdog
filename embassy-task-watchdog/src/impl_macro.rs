#[macro_export]
/// Helper macro to implement the TaskWatchdog and WatchdogRunner for a family of watchdogs.
macro_rules! impl_watchdog {
    ($Family: ident) => {
        use paste::paste;
        paste!{
            use $crate::{MAX_TASKS, runtime::WatchdogOwner};
            /// The WatchdogRunner for this family of watchdogs.  This is the struct you pass to the [`watchdog_run`] function.
            pub struct [<$Family WatchdogRunner>]<const N: usize = MAX_TASKS> {
                runner: &'static WatchdogOwner<N, [<$Family Watchdog>]>,
            }

            impl<const N: usize> WatchdogOwner<N, [<$Family Watchdog>]> {
                /// Used to create a watchdog task when not using the alloc feature.
                ///
                /// There is an equivalent version of this when using the `alloc` feature
                /// which does not include the `const N: usize` type.
                pub(crate) fn create_task(&'static self) -> [<$Family WatchdogRunner>]<N> {
                    [<$Family WatchdogRunner>] { runner: self }
                }
            }

            /// Watchdog Runner function, which will monitor tasks and reset the
            /// system if any.
            ///
            /// You must call this function from an async task to start and run the
            /// watchdog.  Using `spawner.must_spawn(watchdog_run(watchdog))` would
            /// likely be a good choice.
            pub async fn watchdog_run<const N: usize>(task: [<$Family WatchdogRunner>]<N>) -> ! {
                info!("Watchdog runner started");

                // Start the watchdog
                task.runner.start().await;

                // Get initial check interval
                let interval = task.runner.get_check_interval().await;
                let mut check_time = Instant::now() + interval;

                loop {
                    // Check for starved tasks.  We don't do anthing based on the
                    // return code as check_tasks() handles feeding/starving the
                    // hardware watchdog.
                    let _ = task.runner.check_tasks().await;

                    // Wait before checking again
                    Timer::at(check_time).await;
                    check_time += interval;
                }
            }

            use super::TaskDesc;

            /// A per-task bound handle that lets the task call `feed()` without IDs.
            #[doc(hidden)]
            pub struct [<$Family TaskWatchdogInner>]<'a, const N: usize>
            where
                'a: 'static,
            {
                runner: &'a WatchdogOwner<N, [<$Family Watchdog>]>,
                id: u32,
            }

            impl<'a, const N: usize> [<$Family TaskWatchdogInner>]<'a, N> {
                #[inline(always)]
                pub(crate) fn new(runner: &'a WatchdogOwner<N, [<$Family Watchdog>]>, id: u32) -> Self {
                    Self { runner, id }
                }

                #[inline(always)]
                pub async fn feed(&self) {
                    self.runner.feed(self.id).await
                }

                #[inline(always)]
                pub async fn deregister(&self) {
                    self.runner.deregister_task(self.id).await
                }

                #[inline(always)]
                pub async fn reset_reason(&self) -> Option<ResetReason> {
                    self.runner.reset_reason().await
                }

                #[inline(always)]
                pub async fn trigger_reset(&self) -> ! {
                    self.runner.trigger_reset().await
                }
            }

            /// The WatchdogSetup for this family of watchdogs.  This is the struct you create with `new()` and pass to the `build()` function to get the WatchdogRunner and TaskWatchdog.
            pub struct [<$Family WatchdogSetup>]<const N: usize = MAX_TASKS> {
                inner: WatchdogOwner<N, [<$Family Watchdog>]>,
            }

            impl<const N: usize> [<$Family WatchdogSetup>]<N> {
                #[inline(always)]
                #[must_use]
                /// Build the WatchdogRunner and TaskWatchdog for this family of watchdogs.
                pub fn build(&'static self) -> ([<$Family TaskWatchdog>]<N>, [<$Family WatchdogRunner>]<N>) {
                    let iface = [<$Family TaskWatchdog>] { inner: &self.inner };
                    let task = self.create_task();
                    (iface, task)
                }

                // If you want to expose other runner methods, forward them:
                #[inline(always)]
                #[must_use]
                fn create_task(&'static self) -> [<$Family WatchdogRunner>]<N> {
                    // use your existing create_task() on inner
                    // (we need a &'static self; enforce via caller)
                    // SAFETY: self is &'static in signature.
                    let inner: &'static WatchdogOwner<N, [<$Family Watchdog>]> =
                        unsafe { &*(&self.inner as *const _) };
                    inner.create_task()
                }
            }

            #[derive(Clone, Copy)]
            /// A per-task bound handle that lets the task call [`feed`] without IDs.
            ///
            /// Pass a static reference to this struct to the task, decorated by
            /// [`embassy_task_watchdog::task`] as the first argument.
            pub struct [<$Family TaskWatchdog>]<const N: usize = MAX_TASKS> {
                inner: &'static WatchdogOwner<N, [<$Family Watchdog>]>,
            }

            impl<const N: usize> [<$Family TaskWatchdog>]<N> {
                #[inline(always)]
                #[doc(hidden)]
                pub async fn register_desc(
                    self,
                    desc: &'static TaskDesc,
                    max_duration: embassy_time::Duration,
                ) -> [<$Family TaskWatchdogInner>]<'static, N> {
                    self.inner.register_task(desc.id, desc.name, max_duration).await;
                    [<$Family TaskWatchdogInner>]::new(self.inner, desc.id)
                }
            }

            // Re-export for macro path convenience
            pub use [<$Family TaskWatchdog>] as TaskWatchdog;
            pub use [<$Family WatchdogSetup>] as Watchdog;

        }
    }
}
