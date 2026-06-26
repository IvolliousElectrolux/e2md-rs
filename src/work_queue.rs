#![allow(dead_code)]

use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::models::{QueueItem, QueueItemStatus, QueuePoolType};

pub type SharedQueue = Arc<Mutex<Vec<QueueItem>>>;

/// The dual work queue (convert + clean pools).
#[derive(Clone)]
pub struct WorkQueue {
    pub convert_queue: SharedQueue,
    pub clean_queue: SharedQueue,
    convert_semaphore: Arc<Semaphore>,
    clean_semaphore: Arc<Semaphore>,
}

impl WorkQueue {
    pub fn new(max_convert: usize, max_clean: usize) -> Self {
        Self {
            convert_queue: Arc::new(Mutex::new(Vec::new())),
            clean_queue: Arc::new(Mutex::new(Vec::new())),
            convert_semaphore: Arc::new(Semaphore::new(max_convert)),
            clean_semaphore: Arc::new(Semaphore::new(max_clean)),
        }
    }

    pub fn enqueue_convert(&self, item: QueueItem) {
        self.convert_queue.lock().push(item);
    }

    pub fn enqueue_clean(&self, item: QueueItem) {
        self.clean_queue.lock().push(item);
    }

    pub fn get_convert_items(&self) -> Vec<QueueItem> {
        self.convert_queue.lock().clone()
    }

    pub fn get_clean_items(&self) -> Vec<QueueItem> {
        self.clean_queue.lock().clone()
    }

    pub fn cancel(&self, item_id: &str) {
        let mut q = self.convert_queue.lock();
        if let Some(item) = q.iter_mut().find(|i| i.id == item_id) {
            item.status = QueueItemStatus::Cancelled;
            return;
        }
        drop(q);
        let mut q = self.clean_queue.lock();
        if let Some(item) = q.iter_mut().find(|i| i.id == item_id) {
            item.status = QueueItemStatus::Cancelled;
        }
    }

    pub fn clear_done(&self) {
        let mut q = self.convert_queue.lock();
        q.retain(|i| !matches!(i.status, QueueItemStatus::Done | QueueItemStatus::Cancelled));
        drop(q);
        let mut q = self.clean_queue.lock();
        q.retain(|i| !matches!(i.status, QueueItemStatus::Done | QueueItemStatus::Cancelled));
    }

    pub fn move_up(&self, pool: QueuePoolType, item_id: &str) {
        let mut q = match pool {
            QueuePoolType::Convert => self.convert_queue.lock(),
            QueuePoolType::Clean => self.clean_queue.lock(),
        };
        if let Some(pos) = q.iter().position(|i| i.id == item_id) {
            if pos > 0 {
                q.swap(pos, pos - 1);
            }
        }
    }

    pub fn move_down(&self, pool: QueuePoolType, item_id: &str) {
        let mut q = match pool {
            QueuePoolType::Convert => self.convert_queue.lock(),
            QueuePoolType::Clean => self.clean_queue.lock(),
        };
        if let Some(pos) = q.iter().position(|i| i.id == item_id) {
            if pos + 1 < q.len() {
                q.swap(pos, pos + 1);
            }
        }
    }

    /// Acquire a slot from the convert semaphore (async, non-blocking call site).
    pub fn convert_semaphore(&self) -> Arc<Semaphore> {
        Arc::clone(&self.convert_semaphore)
    }

    /// Acquire a slot from the clean semaphore.
    pub fn clean_semaphore(&self) -> Arc<Semaphore> {
        Arc::clone(&self.clean_semaphore)
    }

    pub fn update_item_status(&self, pool: QueuePoolType, item_id: &str, status: QueueItemStatus) {
        let mut q = match pool {
            QueuePoolType::Convert => self.convert_queue.lock(),
            QueuePoolType::Clean => self.clean_queue.lock(),
        };
        if let Some(item) = q.iter_mut().find(|i| i.id == item_id) {
            item.status = status;
        }
    }

    pub fn update_item_progress(&self, pool: QueuePoolType, item_id: &str, progress: f32) {
        let mut q = match pool {
            QueuePoolType::Convert => self.convert_queue.lock(),
            QueuePoolType::Clean => self.clean_queue.lock(),
        };
        if let Some(item) = q.iter_mut().find(|i| i.id == item_id) {
            item.progress = progress;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::QueueItem;

    fn make_item(id: &str, pool: QueuePoolType) -> QueueItem {
        let mut item = QueueItem::new("guid-1", "test.pdf", pool);
        item.id = id.to_string();
        item
    }

    #[test]
    fn enqueue_and_get() {
        let wq = WorkQueue::new(4, 2);
        wq.enqueue_convert(make_item("a", QueuePoolType::Convert));
        wq.enqueue_convert(make_item("b", QueuePoolType::Convert));
        assert_eq!(wq.get_convert_items().len(), 2);
        assert_eq!(wq.get_clean_items().len(), 0);
    }

    #[test]
    fn cancel_sets_status() {
        let wq = WorkQueue::new(4, 2);
        wq.enqueue_convert(make_item("x", QueuePoolType::Convert));
        wq.cancel("x");
        let items = wq.get_convert_items();
        assert_eq!(items[0].status, QueueItemStatus::Cancelled);
    }

    #[test]
    fn move_up_reorders() {
        let wq = WorkQueue::new(4, 2);
        wq.enqueue_convert(make_item("first", QueuePoolType::Convert));
        wq.enqueue_convert(make_item("second", QueuePoolType::Convert));
        wq.move_up(QueuePoolType::Convert, "second");
        let items = wq.get_convert_items();
        assert_eq!(items[0].id, "second");
        assert_eq!(items[1].id, "first");
    }

    #[test]
    fn move_down_reorders() {
        let wq = WorkQueue::new(4, 2);
        wq.enqueue_convert(make_item("alpha", QueuePoolType::Convert));
        wq.enqueue_convert(make_item("beta", QueuePoolType::Convert));
        wq.move_down(QueuePoolType::Convert, "alpha");
        let items = wq.get_convert_items();
        assert_eq!(items[0].id, "beta");
        assert_eq!(items[1].id, "alpha");
    }

    #[test]
    fn clear_done_removes_finished() {
        let wq = WorkQueue::new(4, 2);
        let mut item = make_item("done_item", QueuePoolType::Convert);
        item.status = QueueItemStatus::Done;
        wq.enqueue_convert(item);
        wq.enqueue_convert(make_item("pending", QueuePoolType::Convert));
        wq.clear_done();
        let items = wq.get_convert_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "pending");
    }
}
