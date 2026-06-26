/// Tests for the WorkQueue

#[cfg(test)]
mod tests {
    use crate::models::{QueueItem, QueueItemStatus, QueuePoolType};
    use crate::work_queue::WorkQueue;

    fn item(id: &str, pool: QueuePoolType) -> QueueItem {
        let mut q = QueueItem::new("guid-1", "test.pdf", pool);
        q.id = id.to_string();
        q
    }

    #[test]
    fn empty_queue_returns_no_items() {
        let wq = WorkQueue::new(4, 2);
        assert!(wq.get_convert_items().is_empty());
        assert!(wq.get_clean_items().is_empty());
    }

    #[test]
    fn enqueue_convert_visible_in_get() {
        let wq = WorkQueue::new(4, 2);
        wq.enqueue_convert(item("a", QueuePoolType::Convert));
        let items = wq.get_convert_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "a");
    }

    #[test]
    fn enqueue_clean_visible_in_get() {
        let wq = WorkQueue::new(4, 2);
        wq.enqueue_clean(item("b", QueuePoolType::Clean));
        let items = wq.get_clean_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "b");
    }

    #[test]
    fn cancel_marks_item_cancelled() {
        let wq = WorkQueue::new(4, 2);
        wq.enqueue_convert(item("x", QueuePoolType::Convert));
        wq.cancel("x");
        let items = wq.get_convert_items();
        assert_eq!(items[0].status, QueueItemStatus::Cancelled);
    }

    #[test]
    fn cancel_nonexistent_does_not_panic() {
        let wq = WorkQueue::new(4, 2);
        wq.cancel("nonexistent"); // Should not panic
    }

    #[test]
    fn move_up_at_beginning_is_noop() {
        let wq = WorkQueue::new(4, 2);
        wq.enqueue_convert(item("first", QueuePoolType::Convert));
        wq.enqueue_convert(item("second", QueuePoolType::Convert));
        wq.move_up(QueuePoolType::Convert, "first");
        let items = wq.get_convert_items();
        assert_eq!(items[0].id, "first"); // Unchanged
    }

    #[test]
    fn move_up_second_item() {
        let wq = WorkQueue::new(4, 2);
        wq.enqueue_convert(item("a", QueuePoolType::Convert));
        wq.enqueue_convert(item("b", QueuePoolType::Convert));
        wq.move_up(QueuePoolType::Convert, "b");
        let items = wq.get_convert_items();
        assert_eq!(items[0].id, "b");
        assert_eq!(items[1].id, "a");
    }

    #[test]
    fn move_down_last_item_is_noop() {
        let wq = WorkQueue::new(4, 2);
        wq.enqueue_convert(item("a", QueuePoolType::Convert));
        wq.enqueue_convert(item("b", QueuePoolType::Convert));
        wq.move_down(QueuePoolType::Convert, "b");
        let items = wq.get_convert_items();
        assert_eq!(items[1].id, "b");
    }

    #[test]
    fn move_down_first_item() {
        let wq = WorkQueue::new(4, 2);
        wq.enqueue_convert(item("a", QueuePoolType::Convert));
        wq.enqueue_convert(item("b", QueuePoolType::Convert));
        wq.move_down(QueuePoolType::Convert, "a");
        let items = wq.get_convert_items();
        assert_eq!(items[0].id, "b");
        assert_eq!(items[1].id, "a");
    }

    #[test]
    fn clear_done_removes_done_and_cancelled() {
        let wq = WorkQueue::new(4, 2);
        let mut done_item = item("done", QueuePoolType::Convert);
        done_item.status = QueueItemStatus::Done;
        let mut cancelled = item("cancelled", QueuePoolType::Convert);
        cancelled.status = QueueItemStatus::Cancelled;
        wq.enqueue_convert(done_item);
        wq.enqueue_convert(cancelled);
        wq.enqueue_convert(item("running", QueuePoolType::Convert));
        wq.clear_done();
        let items = wq.get_convert_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "running");
    }

    #[test]
    fn update_item_status_works() {
        let wq = WorkQueue::new(4, 2);
        wq.enqueue_clean(item("c", QueuePoolType::Clean));
        wq.update_item_status(QueuePoolType::Clean, "c", QueueItemStatus::Running);
        let items = wq.get_clean_items();
        assert_eq!(items[0].status, QueueItemStatus::Running);
    }

    #[test]
    fn update_item_progress_works() {
        let wq = WorkQueue::new(4, 2);
        wq.enqueue_convert(item("p", QueuePoolType::Convert));
        wq.update_item_progress(QueuePoolType::Convert, "p", 0.75);
        let items = wq.get_convert_items();
        assert!((items[0].progress - 0.75).abs() < 1e-6);
    }
}
