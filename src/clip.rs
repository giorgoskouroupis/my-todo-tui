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

    #[allow(dead_code)]
    pub fn copy(&mut self, item: TodoItem) {
        self.items.push(item);
    }

    #[allow(dead_code)]
    pub fn paste(&mut self) -> Option<TodoItem> {
        self.items.pop()
    }

    #[allow(dead_code)]
    pub fn has_items(&self) -> bool {
        !self.items.is_empty()
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.items.clear();
    }
}
