/// A linear undo and redo stack.
///
/// `UndoStack` manages history using two internal stacks: one for active/applied
/// items and one for undone items waiting to be redone.
///
/// - [`push`](UndoStack::push) records a new action and clears the redo stack.
/// - [`undo`](UndoStack::undo) moves the most recent action to the redo stack.
/// - [`redo`](UndoStack::redo) moves the most recently undone action back to the active stack.
///
/// # Examples
///
/// ```ignore
/// let mut undo_stack = UndoStack::new();
///
/// undo_stack.push("action 1");
/// undo_stack.push("action 2");
///
/// undo_stack.undo(); // undoes "action 2"
/// undo_stack.redo(); // redoes "action 2"
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct UndoStack<T> {
    stack: Vec<T>,
    redo_stack: Vec<T>,
}

impl<T> Default for UndoStack<T> {
    /// Creates an empty `UndoStack`.
    fn default() -> Self {
        Self::new()
    }
}

impl<T> UndoStack<T> {
    /// Creates a new, empty `UndoStack`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let undo_stack: UndoStack<i32> = UndoStack::new();
    /// ```
    pub fn new() -> Self {
        Self {
            stack: vec![],
            redo_stack: vec![],
        }
    }

    /// Pushes a new item onto the undo stack.
    ///
    /// This records a new action and clears the redo stack, as branching
    /// history is not preserved in a linear undo model.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut undo_stack = UndoStack::new();
    /// undo_stack.push(1);
    /// ```
    pub fn push(&mut self, item: T) {
        self.redo_stack.clear();
        self.stack.push(item);
    }

    /// Undoes the most recent action by moving it to the redo stack.
    ///
    /// If there are no actions to undo, this operation is a no-op.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut undo_stack = UndoStack::new();
    /// undo_stack.push(1);
    /// undo_stack.undo();
    /// ```
    pub fn undo(&mut self) {
        if let Some(item) = self.stack.pop() {
            self.redo_stack.push(item);
        }
    }

    /// Redoes the most recently undone action by moving it back to the undo stack.
    ///
    /// Returns a reference to the redone item, or `None` if there are no actions to redo.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut undo_stack = UndoStack::new();
    /// undo_stack.push(1);
    /// undo_stack.undo();
    ///
    /// assert_eq!(undo_stack.redo(), Some(&1));
    /// assert_eq!(undo_stack.redo(), None);
    /// ```
    pub fn redo(&mut self) -> Option<&T> {
        self.redo_stack.pop().and_then(|item| {
            self.stack.push(item);
            self.stack.last()
        })
    }

    /// Returns a slice of the active items in the undo stack in chronological order.
    ///
    /// The items are ordered from oldest to most recently applied. Undone items
    /// waiting to be redone are not included.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut undo_stack = UndoStack::new();
    /// undo_stack.push(1);
    /// undo_stack.push(2);
    ///
    /// assert_eq!(undo_stack.history(), &[1, 2]);
    ///
    /// undo_stack.undo();
    /// assert_eq!(undo_stack.history(), &[1]);
    /// ```
    pub fn history(&self) -> &[T] {
        &self.stack
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_empty() {
        let undo_stack: UndoStack<i32> = UndoStack::new();
        assert!(undo_stack.stack.is_empty());
        assert!(undo_stack.redo_stack.is_empty());
    }

    #[test]
    fn test_default_is_empty() {
        let undo_stack: UndoStack<i32> = UndoStack::default();
        assert_eq!(undo_stack, UndoStack::new());
    }

    #[test]
    fn test_push() {
        let mut undo_stack = UndoStack::new();
        undo_stack.push(1);
        undo_stack.push(2);

        assert_eq!(undo_stack.stack, vec![1, 2]);
        assert!(undo_stack.redo_stack.is_empty());
    }

    #[test]
    fn test_undo() {
        let mut undo_stack = UndoStack::new();
        undo_stack.push(1);
        undo_stack.push(2);

        undo_stack.undo();
        assert_eq!(undo_stack.stack, vec![1]);
        assert_eq!(undo_stack.redo_stack, vec![2]);

        undo_stack.undo();
        assert!(undo_stack.stack.is_empty());
        assert_eq!(undo_stack.redo_stack, vec![2, 1]);
    }

    #[test]
    fn test_undo_on_empty_stack_is_noop() {
        let mut undo_stack: UndoStack<i32> = UndoStack::new();
        undo_stack.undo();

        assert!(undo_stack.stack.is_empty());
        assert!(undo_stack.redo_stack.is_empty());
    }

    #[test]
    fn test_redo() {
        let mut undo_stack = UndoStack::new();
        undo_stack.push(1);
        undo_stack.push(2);

        undo_stack.undo();
        undo_stack.undo();

        assert_eq!(undo_stack.redo(), Some(&1));
        assert_eq!(undo_stack.stack, vec![1]);
        assert_eq!(undo_stack.redo_stack, vec![2]);

        assert_eq!(undo_stack.redo(), Some(&2));
        assert_eq!(undo_stack.stack, vec![1, 2]);
        assert!(undo_stack.redo_stack.is_empty());
    }

    #[test]
    fn test_redo_on_empty_redo_stack_is_noop() {
        let mut undo_stack = UndoStack::new();
        undo_stack.push(1);

        assert_eq!(undo_stack.redo(), None);
        assert_eq!(undo_stack.stack, vec![1]);
        assert!(undo_stack.redo_stack.is_empty());

        let mut empty_stack: UndoStack<i32> = UndoStack::new();
        assert_eq!(empty_stack.redo(), None);
        assert!(empty_stack.stack.is_empty());
        assert!(empty_stack.redo_stack.is_empty());
    }

    #[test]
    fn test_push_clears_redo_stack() {
        let mut undo_stack = UndoStack::new();
        undo_stack.push(1);
        undo_stack.push(2);

        undo_stack.undo();
        assert_eq!(undo_stack.redo_stack, vec![2]);

        // Pushing a new item clears redo history
        undo_stack.push(3);
        assert_eq!(undo_stack.stack, vec![1, 3]);
        assert!(undo_stack.redo_stack.is_empty());

        // Redo should now be a no-op returning None
        assert_eq!(undo_stack.redo(), None);
        assert_eq!(undo_stack.stack, vec![1, 3]);
        assert!(undo_stack.redo_stack.is_empty());
    }

    #[test]
    fn test_multiple_cycles() {
        let mut undo_stack = UndoStack::new();
        undo_stack.push("a");
        undo_stack.push("b");
        undo_stack.push("c");

        undo_stack.undo(); // pop 'c'
        undo_stack.undo(); // pop 'b'
        assert_eq!(undo_stack.stack, vec!["a"]);

        assert_eq!(undo_stack.redo(), Some(&"b")); // restore 'b'
        assert_eq!(undo_stack.stack, vec!["a", "b"]);

        undo_stack.push("d"); // branch: clears redo ('c' discarded)
        assert_eq!(undo_stack.stack, vec!["a", "b", "d"]);
        assert!(undo_stack.redo_stack.is_empty());

        assert_eq!(undo_stack.redo(), None); // no-op returning None
        assert_eq!(undo_stack.stack, vec!["a", "b", "d"]);
    }

    #[test]
    fn test_history_is_empty_initially() {
        let undo_stack: UndoStack<i32> = UndoStack::new();
        assert_eq!(undo_stack.history(), &[]);
    }

    #[test]
    fn test_history_reflects_pushed_items() {
        let mut undo_stack = UndoStack::new();
        undo_stack.push(10);
        assert_eq!(undo_stack.history(), &[10]);

        undo_stack.push(20);
        assert_eq!(undo_stack.history(), &[10, 20]);

        undo_stack.push(30);
        assert_eq!(undo_stack.history(), &[10, 20, 30]);
    }

    #[test]
    fn test_history_with_undo_and_redo() {
        let mut undo_stack = UndoStack::new();
        undo_stack.push("alpha");
        undo_stack.push("beta");
        undo_stack.push("gamma");

        undo_stack.undo();
        assert_eq!(undo_stack.history(), &["alpha", "beta"]);

        undo_stack.undo();
        assert_eq!(undo_stack.history(), &["alpha"]);

        assert_eq!(undo_stack.redo(), Some(&"beta"));
        assert_eq!(undo_stack.history(), &["alpha", "beta"]);

        // Branching clears redo and appends new item to history
        undo_stack.push("delta");
        assert_eq!(undo_stack.history(), &["alpha", "beta", "delta"]);
        assert_eq!(undo_stack.redo(), None);
    }
}
