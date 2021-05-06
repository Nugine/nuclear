use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::sync::Arc;

thread_local! {
    static CURRENT_STATE: RefCell<Option<Arc<dyn Any+Send+Sync+'static>>> = RefCell::new(None);
}

pub fn enter<S, R>(state: Arc<S>, f: impl FnOnce() -> R) -> R
where
    S: Send + Sync + 'static,
{
    struct Guard<'a> {
        prev: Option<Arc<dyn Any + Send + Sync + 'static>>,
        cell: &'a RefCell<Option<Arc<dyn Any + Send + Sync + 'static>>>,
    }

    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            *self.cell.borrow_mut() = self.prev.take()
        }
    }

    CURRENT_STATE.with(move |cell| {
        let prev = cell.replace(Some(state));
        let _guard = Guard { prev, cell };
        f()
    })
}

pub fn inject<S>() -> Option<Arc<S>>
where
    S: Send + Sync + 'static,
{
    CURRENT_STATE.with(move |cell| {
        cell.borrow().as_ref().and_then(|s| {
            if Any::type_id(&**s) == TypeId::of::<S>() {
                Some(Arc::downcast(s.clone()).unwrap())
            } else {
                None
            }
        })
    })
}
