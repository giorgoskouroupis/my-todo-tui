use crate::data::TodoItem;

#[derive(Default)]
pub struct Clipboard {
    items: Vec<TodoItem>,
}

impl Clipboard {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn cut(&mut self, item: TodoItem) {
        self.items.push(item);
    }

    pub fn copy(&mut self, item: TodoItem) {
        self.items.push(item);
    }

    pub fn paste(&mut self) -> Option<TodoItem> {
        self.items.pop()
    }

    pub fn has_items(&self) -> bool {
        !self.items.is_empty()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}
