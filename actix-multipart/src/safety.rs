use std::{cell::Cell, marker::PhantomData, sync::Arc, task};

use local_waker::LocalWaker;

/// Counter. It tracks of number of clones of payloads and give access to payload only to top most.
///
/// - When dropped, parent task is awakened. This is to support the case where `Field` is dropped in
///   a separate task than `Multipart`.
/// - Assumes that parent owners don't move to different tasks; only the top-most is allowed to.
/// - If dropped and is not top most owner, is_clean flag is set to false.
#[derive(Debug)]
pub(crate) struct Safety {
    task: LocalWaker,
    level: usize,
    payload: Arc<PhantomData<bool>>,
    clean: Arc<Cell<bool>>,
}

impl Safety {
    pub(crate) fn new() -> Safety {
        let payload = Arc::new(PhantomData);
        Safety {
            task: LocalWaker::new(),
            level: Arc::strong_count(&payload),
            clean: Arc::new(Cell::new(true)),
            payload,
        }
    }

    pub(crate) fn current(&self) -> bool {
        Arc::strong_count(&self.payload) == self.level && self.clean.get()
    }

    pub(crate) fn is_clean(&self) -> bool {
        self.clean.get()
    }

    pub(crate) fn clone(&self, cx: &task::Context<'_>) -> Safety {
        let payload = Arc::clone(&self.payload);
        let s = Safety {
            task: LocalWaker::new(),
            level: Arc::strong_count(&payload),
            clean: self.clean.clone(),
            payload,
        };
        s.task.register(cx.waker());
        s
    }
}

impl Drop for Safety {
    fn drop(&mut self) {
        if Arc::strong_count(&self.payload) != self.level {
            // Multipart dropped leaving a Field
            self.clean.set(false);
        }

        self.task.wake();
    }
}
