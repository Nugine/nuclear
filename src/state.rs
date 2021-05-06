#![allow(unsafe_code)]

use std::any::TypeId;
use std::cell::Cell;
use std::ptr::NonNull;
use std::sync::Arc;

thread_local! {
    static CURRENT_STATE: Cell<Option<ErasedStateRef>> = Cell::new(None)
}

#[derive(Clone, Copy)]
struct ErasedStateRef {
    ptr: NonNull<()>, // &Arc<S>
    id: TypeId,
}

pub fn enter<S, R>(state: &Arc<S>, f: impl FnOnce() -> R) -> R
where
    S: Send + Sync + 'static,
{
    struct Guard<'a> {
        cell: &'a Cell<Option<ErasedStateRef>>,
        prev: Option<ErasedStateRef>,
    }

    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            self.cell.set(self.prev)
        }
    }

    CURRENT_STATE.with(|cell| {
        let prev = cell.replace(Some(ErasedStateRef {
            ptr: NonNull::from(state).cast(),
            id: TypeId::of::<S>(),
        }));
        let _guard = Guard { cell, prev };
        f()
    })
}

pub fn inject<S>() -> Option<Arc<S>>
where
    S: Send + Sync + 'static,
{
    CURRENT_STATE.with(|cell| {
        cell.get()
            .filter(|s| s.id == TypeId::of::<S>())
            .map(|s| unsafe { Arc::clone(s.ptr.cast::<Arc<S>>().as_ref()) })
    })
}
