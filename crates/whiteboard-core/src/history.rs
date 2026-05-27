use crate::commands::{apply_command, Command};
use crate::document::WhiteboardDoc;

#[derive(Debug, Clone)]
struct CommandPair {
    redo: Command,
    undo: Command,
}

pub struct History {
    undo_stack: Vec<CommandPair>,
    redo_stack: Vec<CommandPair>,
    capacity: usize,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            capacity: capacity.max(1),
        }
    }

    pub fn apply(&mut self, doc: &mut WhiteboardDoc, command: Command) {
        let Some(undo) = apply_command(doc, &command) else {
            return;
        };

        if self.undo_stack.len() >= self.capacity {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(CommandPair {
            redo: command,
            undo,
        });
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, doc: &mut WhiteboardDoc) -> bool {
        let Some(pair) = self.undo_stack.pop() else {
            return false;
        };

        if apply_command(doc, &pair.undo).is_none() {
            return false;
        }
        self.redo_stack.push(pair);
        true
    }

    pub fn redo(&mut self, doc: &mut WhiteboardDoc) -> bool {
        let Some(pair) = self.redo_stack.pop() else {
            return false;
        };

        if apply_command(doc, &pair.redo).is_none() {
            return false;
        }
        self.undo_stack.push(pair);
        true
    }
}
