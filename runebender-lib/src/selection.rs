use std::collections::BTreeSet;
use std::sync::Arc;

use druid::Data;

use crate::point::EntityId;

/// A sorted set of selected items.
#[derive(Debug, Clone, Data)]
pub struct Selection {
    items: Arc<BTreeSet<EntityId>>,
}

impl Selection {
    pub fn new() -> Selection {
        Selection {
            items: Arc::new(BTreeSet::new()),
        }
    }

    /// Returns the number of items in the selection.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if there are no selected items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterate the selected ids.
    pub fn iter(&self) -> impl Iterator<Item = &EntityId> {
        self.items.iter()
    }

    /// Returns a [`PathSelection`] object that can be used to iterate
    /// through the selection, grouped by path.
    pub fn per_path_selection(&self) -> PathSelection {
        PathSelection::new(&self.items)
    }

    /// Add an item to the selection.
    ///
    /// If the selection did not previously contain this item, `true` is returned.
    pub fn insert(&mut self, item: EntityId) -> bool {
        if self.items.contains(&item) {
            false
        } else {
            assert!(self.items_mut().insert(item));
            true
        }
    }

    /// Remove an item from the selection.
    ///
    /// Returns `true` if the item was present in the selection.
    pub fn remove(&mut self, item: &EntityId) -> bool {
        if !self.items.contains(item) {
            false
        } else {
            assert!(self.items_mut().remove(item));
            true
        }
    }

    /// Returns `true` if the selection contains this item.
    pub fn contains(&self, item: &EntityId) -> bool {
        self.items.contains(item)
    }

    /// Set the selection to contain this single item.
    pub fn select_one(&mut self, item: EntityId) {
        let items = self.items_mut();
        items.clear();
        items.insert(item);
    }

    /// Clear the selection.
    pub fn clear(&mut self) {
        if !self.is_empty() {
            self.items_mut().clear()
        }
    }

    /// The symmetric_difference of two selections.
    pub fn symmetric_difference(&self, other: &Selection) -> Selection {
        self.items
            .symmetric_difference(&other.items)
            .copied()
            .collect()
    }

    /// The union of two selections.
    pub fn union(&self, other: &Selection) -> Selection {
        self.items.union(&other.items).copied().collect()
    }

    fn items_mut(&mut self) -> &mut BTreeSet<EntityId> {
        Arc::make_mut(&mut self.items)
    }
}

/// A helper for iterating through a selection in per-path chunks.
///
/// To iterate, call the `iter` method; this object is reuseable.
pub struct PathSelection {
    inner: Vec<EntityId>,
}

impl PathSelection {
    fn new(src: &BTreeSet<EntityId>) -> PathSelection {
        let mut inner: Vec<_> = src.iter().copied().collect();
        inner.sort();
        PathSelection { inner }
    }

    pub fn iter(&self) -> PathSelectionIter {
        PathSelectionIter {
            inner: &self.inner,
            idx: 0,
        }
    }

    /// The number of distinct paths represented in the selection.
    pub fn path_len(&self) -> usize {
        self.inner
            .iter()
            .fold((0, EntityId::next()), |(len, prev), id| {
                let len = if id.parent_eq(prev) { len } else { len + 1 };
                (len, *id)
            })
            .0
    }
}

pub struct PathSelectionIter<'a> {
    inner: &'a [EntityId],
    idx: usize,
}

impl<'a> Iterator for PathSelectionIter<'a> {
    type Item = &'a [EntityId];
    fn next(&mut self) -> Option<&'a [EntityId]> {
        if self.idx >= self.inner.len() {
            return None;
        }
        let path_id = self.inner[self.idx];
        let end_idx = self.inner[self.idx..]
            .iter()
            .position(|p| !p.parent_eq(path_id))
            .map(|idx| idx + self.idx)
            .unwrap_or_else(|| self.inner.len());
        let range = self.idx..end_idx;
        self.idx = end_idx;
        // probably unnecessary, but we don't expect empty slices
        if range.start == range.end {
            None
        } else {
            Some(&self.inner[range])
        }
    }
}

impl std::iter::Extend<EntityId> for Selection {
    fn extend<T: IntoIterator<Item = EntityId>>(&mut self, iter: T) {
        self.items_mut().extend(iter)
    }
}

impl std::iter::FromIterator<EntityId> for Selection {
    fn from_iter<T: IntoIterator<Item = EntityId>>(iter: T) -> Self {
        Selection {
            items: Arc::new(iter.into_iter().collect()),
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Selection::new()
    }
}
