use embassy_time::Duration;

/// Descriptor emitted by the proc-macro (NOT in a linker section).
#[repr(C)]
#[doc(hidden)]
pub struct TaskDesc {
    pub name: &'static str,
}

/// Auto-generated ID type used internally by the auto-watchdog path.
///
/// We derive it from the address of a unique `static TaskDesc`.
/// It's Copy + Eq + Debug (and defmt::Format when enabled).
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[doc(hidden)]
pub struct TaskKey(u32);

impl core::fmt::Debug for TaskKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Show as hex; stable and useful
        write!(f, "TaskKey(0x{:08x})", self.0)
    }
}

impl TaskKey {
    /// Derive a stable-ish key for this boot from a descriptor's address.
    ///
    /// We hash the pointer down to u32 to keep the ID small.
    /// Collisions are extremely unlikely with a small number of tasks.
    #[inline(always)]
    #[doc(hidden)]
    pub fn from_desc(desc: &'static TaskDesc) -> Self {
        let addr = (desc as *const TaskDesc as usize) as u64;

        // A tiny fixed hash (no alloc, no std). Use splitmix64 then truncate.
        let mut x = addr.wrapping_add(0x9E3779B97F4A7C15);
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
        x ^= x >> 31;

        TaskKey((x as u32) ^ ((x >> 32) as u32))
    }
}

/// A per-task bound handle that lets the task call `feed()` without IDs.
pub struct BoundWatchdog<'a, const N: usize>
where
    'a: 'static,
{
    runner: &'a crate::embassy_rp::RpWatchdogOwner<N>,
    id: TaskKey,
}

impl<'a, const N: usize> BoundWatchdog<'a, N> {
    #[inline(always)]
    pub(crate) fn new(runner: &'a crate::embassy_rp::RpWatchdogOwner<N>, id: TaskKey) -> Self {
        Self { runner, id }
    }

    #[inline(always)]
    pub async fn feed(&self) {
        self.runner.feed(&self.id).await
    }

    #[inline(always)]
    pub async fn deregister(&self) {
        self.runner.deregister_task(&self.id).await
    }

    #[inline(always)]
    pub(crate) fn id(&self) -> TaskKey {
        self.id
    }
}