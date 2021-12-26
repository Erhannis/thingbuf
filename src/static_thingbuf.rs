use crate::loom::atomic::Ordering;
use crate::{Core, Full, Ref, Slot};
use core::{fmt, ptr};

pub struct StaticThingBuf<T, const CAP: usize> {
    core: Core,
    slots: [Slot<T>; CAP],
}

// === impl ThingBuf ===

#[cfg(not(test))]
impl<T, const CAP: usize> StaticThingBuf<T, CAP> {
    pub const fn new() -> Self {
        Self {
            core: Core::new(CAP),
            slots: Slot::make_static_array::<CAP>(),
        }
    }
}

impl<T, const CAP: usize> StaticThingBuf<T, CAP> {
    #[inline]
    pub fn capacity(&self) -> usize {
        CAP
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.core.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T: Default, const CAP: usize> StaticThingBuf<T, CAP> {
    pub fn push_ref(&self) -> Result<Ref<'_, T>, Full> {
        self.core.push_ref(&self.slots).map_err(|e| match e {
            crate::mpsc::TrySendError::Full(()) => Full(()),
            _ => unreachable!(),
        })
    }

    #[inline]
    pub fn push_with<U>(&self, f: impl FnOnce(&mut T) -> U) -> Result<U, Full> {
        self.push_ref().map(|mut r| r.with_mut(f))
    }

    pub fn pop_ref(&self) -> Option<Ref<'_, T>> {
        self.core.pop_ref(&self.slots).ok()
    }

    #[inline]
    pub fn pop_with<U>(&self, f: impl FnOnce(&mut T) -> U) -> Option<U> {
        self.pop_ref().map(|mut r| r.with_mut(f))
    }
}

impl<T, const CAP: usize> Drop for StaticThingBuf<T, CAP> {
    fn drop(&mut self) {
        let tail = self.core.tail.load(Ordering::SeqCst);
        let (idx, gen) = self.core.idx_gen(tail);
        let num_initialized = if gen > 0 { self.capacity() } else { idx };
        for slot in &self.slots[..num_initialized] {
            unsafe {
                slot.value
                    .with_mut(|value| ptr::drop_in_place((*value).as_mut_ptr()));
            }
        }
    }
}

impl<T, const CAP: usize> fmt::Debug for StaticThingBuf<T, CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticThingBuf")
            .field("len", &self.len())
            .field("slots", &format_args!("[...]"))
            .field("core", &self.core)
            .finish()
    }
}
