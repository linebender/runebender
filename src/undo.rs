//! Managing undo state

use std::collections::VecDeque;

// for no good reason
const DEFAULT_UNDO_STACK_SIZE: usize = 128;

/// A stack of states that can be undone and redone.
#[derive(Debug)]
pub(crate) struct UndoState<T> {
    max_undo_count: usize,
    stack: VecDeque<T>,
    /// The index in `stack` of the current document.
    live_index: usize,
}

impl<T> UndoState<T> {
    pub(crate) fn new(init_state: T) -> Self {
        Self::new_sized(DEFAULT_UNDO_STACK_SIZE, init_state)
    }

    fn new_sized(max_undo_count: usize, init_state: T) -> Self {
        let mut stack = VecDeque::new();
        stack.push_back(init_state);
        UndoState {
            max_undo_count,
            stack,
            live_index: 0,
        }
    }

    pub(crate) fn undo(&mut self) -> Option<&T> {
        if self.live_index == 0 {
            return None;
        }
        self.live_index -= 1;
        self.stack.get(self.live_index)
    }

    pub(crate) fn redo(&mut self) -> Option<&T> {
        if self.live_index == self.stack.len() - 1 {
            return None;
        }
        self.live_index += 1;
        self.stack.get(self.live_index)
    }

    pub(crate) fn add_undo_group(&mut self, item: T) {
        if self.live_index < self.stack.len() - 1 {
            self.stack.truncate(self.live_index + 1);
        }

        self.live_index += 1;
        self.stack.push_back(item);

        if self.stack.len() > self.max_undo_count {
            self.stack.pop_front();
            self.live_index -= 1;
        }
    }

    /// Modify the state for the currently active undo group.
    /// This might be done if an edit occurs that combines with the previous undo,
    /// or if we want to save selection state.
    pub(crate) fn update_current_undo(&mut self, mut f: impl FnMut(&mut T)) {
        f(self.stack.get_mut(self.live_index).unwrap())
    }
}
