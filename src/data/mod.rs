mod storage;

use serde::{Deserialize, Serialize};

pub use storage::Storage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

impl Priority {
    pub fn next(self) -> Self {
        match self {
            Self::Low => Self::Normal,
            Self::Normal => Self::High,
            Self::High => Self::Urgent,
            Self::Urgent => Self::Low,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Low => Self::Urgent,
            Self::Normal => Self::Low,
            Self::High => Self::Normal,
            Self::Urgent => Self::High,
        }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: u64,
    pub text: String,
    pub done: bool,
    pub doing: bool,
    pub priority: Priority,
    pub order: u64,
    pub pinned: bool,
    pub due_date: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoData {
    next_id: u64,
    items: Vec<TodoItem>,
    #[serde(default)]
    categories: Vec<String>,
}

impl TodoData {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            items: Vec::new(),
            categories: Vec::new(),
        }
    }

    pub fn items(&self) -> &[TodoItem] {
        &self.items
    }

    pub fn pending_count(&self) -> usize {
        self.items.iter().filter(|i| !i.done).count()
    }

    pub fn add(&mut self, text: &str) -> u64 {
        let text = text.trim();
        if text.is_empty() || text.len() > 500 {
            return 0;
        }

        let id = self.next_id;
        self.next_id += 1;

        let order = self
            .items
            .last()
            .map(|i| i.order + 1)
            .unwrap_or(0);

        self.items.push(TodoItem {
            id,
            text: text.to_string(),
            done: false,
            doing: false,
            priority: Priority::Normal,
            order,
            pinned: false,
            due_date: None,
            category: None,
        });

        id
    }

    pub fn restore(&mut self, item: TodoItem) {
        self.items.push(item);
    }

    pub fn update_text(&mut self, id: u64, text: &str) -> bool {
        let text = text.trim();
        if text.is_empty() || text.len() > 500 {
            return false;
        }

        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.text = text.to_string();
            true
        } else {
            false
        }
    }

    pub fn toggle_done(&mut self, id: u64) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.done = !item.done;
            if item.done {
                item.doing = false;
            }
        }
    }

    pub fn toggle_doing(&mut self, id: u64) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.doing = !item.doing;
            if item.doing {
                item.done = false;
            }
        }
    }

    pub fn toggle_pin(&mut self, id: u64) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.pinned = !item.pinned;
        }
    }

    pub fn set_due_date(&mut self, id: u64, date: Option<String>) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.due_date = date;
        }
    }

    pub fn set_category(&mut self, id: u64, category: Option<String>) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.category = category;
        }
    }

    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self.items.iter()
            .filter_map(|i| i.category.clone())
            .collect();
        cats.extend(self.categories.iter().cloned());
        cats.sort();
        cats.dedup();
        cats
    }

    pub fn add_category(&mut self, name: &str) {
        if !self.categories.contains(&name.to_string()) {
            self.categories.push(name.to_string());
        }
    }

    pub fn remove_category(&mut self, name: &str) {
        self.categories.retain(|c| c != name);
    }

    pub fn cycle_priority(&mut self, id: u64, forward: bool) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.priority = if forward {
                item.priority.next()
            } else {
                item.priority.prev()
            };
        }
    }

    pub fn delete(&mut self, id: u64) -> Option<TodoItem> {
        let idx = self.items.iter().position(|i| i.id == id)?;
        Some(self.items.remove(idx))
    }

    pub fn reorder(&mut self, id: u64, direction: i32) -> bool {
        let idx = self.items.iter().position(|i| i.id == id);
        let Some(idx) = idx else { return false };

        let new_idx = idx as i32 + direction;
        if new_idx < 0 || new_idx >= self.items.len() as i32 {
            return false;
        }

        let new_idx = new_idx as usize;
        let order = self.items[new_idx].order;
        self.items[idx].order = order;
        self.items[new_idx].order = if direction > 0 {
            order.wrapping_sub(1)
        } else {
            order.wrapping_add(1)
        };

        self.items.sort_by_key(|i| i.order);
        true
    }

    pub fn clear_done(&mut self) {
        self.items.retain(|i| !i.done);
    }

    pub fn get(&self, id: u64) -> Option<&TodoItem> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn load() -> Self {
        Storage::load().unwrap_or_else(|_| {
            let empty = TodoData::new();
            let _ = Storage::save(&empty);
            empty
        })
    }

    pub fn save(&self) {
        let _ = Storage::save(self);
    }
}
