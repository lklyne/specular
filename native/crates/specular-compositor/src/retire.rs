//! Shared textures the compositor has stopped drawing but the GPU may still
//! be sampling.
//!
//! Dropping a [`SharedTexture`](specular_core::SharedTexture) hands its surface
//! back to the producer's pool, so it must outlive every submission that
//! sampled it or the producer could repaint a surface mid-read. Entries are
//! tagged with the last submission serial that could have used them and are
//! dropped once the GPU reports that serial complete.

use std::collections::VecDeque;

use specular_core::PageId;

#[derive(Debug)]
struct Retired<T> {
    page: PageId,
    after_serial: u64,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "held only so its Drop runs after the GPU is done")
    )]
    item: T,
}

/// FIFO of items waiting for a submission serial to complete.
#[derive(Debug)]
pub(crate) struct RetiredTextures<T> {
    entries: VecDeque<Retired<T>>,
}

impl<T> Default for RetiredTextures<T> {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }
}

impl<T> RetiredTextures<T> {
    /// Holds `item` until submission `after_serial` has completed. Serials
    /// must be pushed in non-decreasing order.
    pub(crate) fn push(&mut self, page: PageId, after_serial: u64, item: T) {
        debug_assert!(
            self.entries
                .back()
                .is_none_or(|last| last.after_serial <= after_serial),
            "retire serials must not go backwards"
        );
        self.entries.push_back(Retired {
            page,
            after_serial,
            item,
        });
    }

    /// Drops every item whose submission is `<= completed`; returns how many.
    pub(crate) fn reclaim(&mut self, completed: u64) -> usize {
        let mut released = 0;
        while self
            .entries
            .front()
            .is_some_and(|entry| entry.after_serial <= completed)
        {
            self.entries.pop_front();
            released += 1;
        }
        released
    }

    /// Items still held for `page`.
    pub(crate) fn count_for(&self, page: PageId) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.page == page)
            .count()
    }

    /// Whether anything is waiting on the GPU.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The items in retirement order, for inspection.
    #[cfg(test)]
    fn items(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().map(|entry| &entry.item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reclaim_releases_only_completed_serials() {
        let mut retired = RetiredTextures::default();
        retired.push(PageId(1), 1, "a");
        retired.push(PageId(1), 2, "b");
        retired.push(PageId(1), 3, "c");
        retired.reclaim(2);
        assert_eq!(retired.items().copied().collect::<Vec<_>>(), ["c"]);
    }

    #[test]
    fn dropping_reclaimed_entries_runs_their_destructors() {
        use std::cell::Cell;
        use std::rc::Rc;
        struct Guard(Rc<Cell<u32>>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        let drops = Rc::new(Cell::new(0));
        let mut retired = RetiredTextures::default();
        retired.push(PageId(1), 5, Guard(Rc::clone(&drops)));
        retired.reclaim(4);
        retired.reclaim(5);
        assert_eq!(drops.get(), 1);
    }
}
