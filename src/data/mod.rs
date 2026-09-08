mod storage;

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::date::Date;

pub use storage::Storage;

pub const NOTE_MAX_LEN: usize = 500;

fn dedup_preserve_order(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

fn category_root(path: &str) -> &str {
    path.split_once('/').map_or(path, |(root, _)| root)
}

/// Keep every category sharing a root contiguous, so a child added long after
/// its siblings still lands in the same group. Roots keep the order they first
/// appear in, and children keep their order inside a group.
fn group_categories(values: &mut Vec<String>) {
    let mut roots: Vec<String> = Vec::new();
    for value in values.iter() {
        let root = category_root(value);
        if !roots.iter().any(|known| known == root) {
            roots.push(root.to_string());
        }
    }

    values.sort_by_key(|value| {
        let root = category_root(value);
        roots.iter().position(|known| known == root)
    });
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub created: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: u64,
    pub text: String,
    pub done: bool,
    #[serde(default)]
    pub doing: bool,
    pub priority: Priority,
    pub order: u64,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryEntry {
    pub path: String,
    pub label: String,
    pub depth: usize,
    pub is_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoData {
    next_id: u64,
    items: Vec<TodoItem>,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    archived_categories: Vec<String>,
}

impl TodoData {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            items: Vec::new(),
            categories: Vec::new(),
            archived_categories: Vec::new(),
        }
    }

    pub fn items(&self) -> &[TodoItem] {
        &self.items
    }

    pub fn pending_count(&self) -> usize {
        self.items.iter().filter(|i| !i.done && !i.archived).count()
    }

    pub fn add(&mut self, text: &str) -> u64 {
        let text = text.trim();
        if text.is_empty() || text.len() > 500 {
            return 0;
        }

        let id = self.next_id;
        self.next_id += 1;

        let order = self.items.last().map(|i| i.order + 1).unwrap_or(0);

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
            archived: false,
            notes: Vec::new(),
        });

        id
    }

    pub fn add_note(&mut self, id: u64, text: &str) -> bool {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > NOTE_MAX_LEN {
            return false;
        }

        match self.items.iter_mut().find(|i| i.id == id) {
            Some(item) => {
                item.notes.push(Note {
                    created: Date::today().iso(),
                    text: text.to_string(),
                });
                true
            }
            None => false,
        }
    }

    pub fn update_note(&mut self, id: u64, index: usize, text: &str) -> bool {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > NOTE_MAX_LEN {
            return false;
        }

        let Some(item) = self.items.iter_mut().find(|i| i.id == id) else {
            return false;
        };
        match item.notes.get_mut(index) {
            Some(note) => {
                note.text = text.to_string();
                true
            }
            None => false,
        }
    }

    pub fn delete_note(&mut self, id: u64, index: usize) -> Option<Note> {
        let item = self.items.iter_mut().find(|i| i.id == id)?;
        if index >= item.notes.len() {
            return None;
        }
        Some(item.notes.remove(index))
    }

    pub fn restore_note(&mut self, id: u64, index: usize, note: Note) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            let index = index.min(item.notes.len());
            item.notes.insert(index, note);
        }
    }

    pub fn note_count(&self, id: u64) -> usize {
        self.get(id).map_or(0, |item| item.notes.len())
    }

    pub fn last_note_index(&self, id: u64) -> usize {
        self.note_count(id).saturating_sub(1)
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
            item.category = category.and_then(|name| normalize_category(&name));
        }
    }

    pub fn set_archived(&mut self, id: u64, archived: bool) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.archived = archived;
        }
    }

    pub fn categories(&self) -> Vec<String> {
        self.categories_for_archived(false)
    }

    pub fn categories_for_archived(&self, archived: bool) -> Vec<String> {
        let stored = if archived {
            &self.archived_categories
        } else {
            &self.categories
        };
        let mut cats: Vec<String> = stored.clone();
        for item in &self.items {
            if item.archived == archived {
                if let Some(ref cat) = item.category {
                    if !cats.contains(cat) {
                        cats.push(cat.clone());
                    }
                }
            }
        }
        group_categories(&mut cats);
        cats
    }

    pub fn category_entries(&self) -> Vec<CategoryEntry> {
        self.category_entries_for_archived(false)
    }

    pub fn category_entries_for_archived(&self, archived: bool) -> Vec<CategoryEntry> {
        let categories = self.categories_for_archived(archived);
        let mut entries = Vec::new();
        let mut i = 0;

        while i < categories.len() {
            let cat = &categories[i];
            let Some((parent, child)) = cat.split_once('/') else {
                let has_children = categories
                    .iter()
                    .any(|other| other.starts_with(&format!("{cat}/")));
                if !has_children {
                    entries.push(CategoryEntry {
                        path: cat.clone(),
                        label: cat.clone(),
                        depth: 0,
                        is_all: false,
                    });
                }
                i += 1;
                continue;
            };

            let parent = parent.to_string();
            entries.push(CategoryEntry {
                path: parent.clone(),
                label: parent.clone(),
                depth: 0,
                is_all: false,
            });

            while i < categories.len() {
                let current = &categories[i];
                let prefix = format!("{parent}/");
                if !current.starts_with(&prefix) {
                    break;
                }
                let label = current.strip_prefix(&prefix).unwrap_or(child).to_string();
                entries.push(CategoryEntry {
                    path: current.clone(),
                    label,
                    depth: 1,
                    is_all: false,
                });
                i += 1;
            }
        }

        entries
    }

    pub fn add_category(&mut self, name: &str) {
        if let Some(name) = normalize_category(name) {
            if !self.categories.contains(&name) {
                self.categories.push(name.clone());
                group_categories(&mut self.categories);
            }
            self.archived_categories.retain(|c| c != &name);
        }
    }

    pub fn remove_category(&mut self, name: &str) {
        self.categories.retain(|c| !category_matches(Some(c), name));
        self.archived_categories
            .retain(|c| !category_matches(Some(c), name));
    }

    pub fn set_category_archived(&mut self, name: &str, archived: bool) {
        let Some(name) = normalize_category(name) else {
            return;
        };

        if archived {
            move_category_branch(&mut self.categories, &mut self.archived_categories, &name);
        } else {
            move_category_branch(&mut self.archived_categories, &mut self.categories, &name);
        }

        for item in &mut self.items {
            if category_matches(item.category.as_deref(), &name) {
                item.archived = archived;
            }
        }
    }

    pub fn rename_category(&mut self, old: &str, new: &str) -> bool {
        let Some(new) = normalize_category(new) else {
            return false;
        };
        let old_prefix = format!("{old}/");
        let mut changed = false;

        for item in &mut self.items {
            if let Some(category) = item.category.as_mut() {
                if category == old {
                    *category = new.clone();
                    changed = true;
                } else if let Some(suffix) = category.strip_prefix(&old_prefix) {
                    *category = format!("{new}/{suffix}");
                    changed = true;
                }
            }
        }

        for category in &mut self.categories {
            if category == old {
                *category = new.clone();
                changed = true;
            } else if let Some(suffix) = category.strip_prefix(&old_prefix) {
                *category = format!("{new}/{suffix}");
                changed = true;
            }
        }
        dedup_preserve_order(&mut self.categories);
        group_categories(&mut self.categories);
        for category in &mut self.archived_categories {
            if category == old {
                *category = new.clone();
                changed = true;
            } else if let Some(suffix) = category.strip_prefix(&old_prefix) {
                *category = format!("{new}/{suffix}");
                changed = true;
            }
        }
        dedup_preserve_order(&mut self.archived_categories);
        group_categories(&mut self.archived_categories);
        changed
    }

    pub fn cycle_priority(&mut self, id: u64) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.priority = item.priority.next();
        }
    }

    pub fn delete(&mut self, id: u64) -> Option<TodoItem> {
        let idx = self.items.iter().position(|i| i.id == id)?;
        Some(self.items.remove(idx))
    }

    pub fn restore_item(&mut self, item: TodoItem) {
        if item.id >= self.next_id {
            self.next_id = item.id + 1;
        }
        if !self.items.iter().any(|existing| existing.id == item.id) {
            self.items.push(item);
            self.items.sort_by_key(|i| i.order);
        }
    }

    pub fn reorder(&mut self, id: u64, direction: i32) -> bool {
        let idx = self.items.iter().position(|i| i.id == id);
        let Some(idx) = idx else { return false };

        let new_idx = idx as i32 + direction;
        if new_idx < 0 || new_idx >= self.items.len() as i32 {
            return false;
        }

        let new_idx = new_idx as usize;
        let a_order = self.items[idx].order;
        let b_order = self.items[new_idx].order;
        self.items[idx].order = b_order;
        self.items[new_idx].order = a_order;

        self.items.sort_by_key(|i| i.order);
        true
    }

    pub fn reorder_category(&mut self, path: &str, direction: i32) -> bool {
        let Some((parent, _)) = path.split_once('/') else {
            return false;
        };
        let idx = match self.categories.iter().position(|c| c == path) {
            Some(i) => i,
            None => return false,
        };
        let prefix = format!("{parent}/");
        let group_start = self.categories[..idx]
            .iter()
            .rposition(|c| !c.starts_with(&prefix))
            .map(|pos| pos + 1)
            .unwrap_or(0);
        let group_end = self.categories[idx + 1..]
            .iter()
            .position(|c| !c.starts_with(&prefix))
            .map(|pos| idx + 1 + pos)
            .unwrap_or(self.categories.len());
        let new_idx = idx as i32 + direction;
        if new_idx < group_start as i32 || new_idx >= group_end as i32 {
            return false;
        }
        self.categories.swap(idx, new_idx as usize);
        true
    }

    pub fn clear_done(&mut self) {
        self.items.retain(|i| !i.done);
    }

    pub fn get(&self, id: u64) -> Option<&TodoItem> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn load() -> Self {
        match Storage::load() {
            Ok(mut data) => {
                // Files written before grouping was enforced can hold scattered
                // siblings; regroup them once so reordering works on them too.
                group_categories(&mut data.categories);
                group_categories(&mut data.archived_categories);
                data
            }
            Err(reason) => {
                Storage::backup_corrupt(&reason);
                let empty = TodoData::new();
                let _ = Storage::save(&empty);
                empty
            }
        }
    }

    pub fn save(&self) {
        let _ = Storage::save(self);
    }
}

fn move_category_branch(from: &mut Vec<String>, to: &mut Vec<String>, name: &str) {
    let mut moved = Vec::new();
    from.retain(|category| {
        if category_matches(Some(category), name) {
            moved.push(category.clone());
            false
        } else {
            true
        }
    });

    if moved.is_empty() {
        moved.push(name.to_string());
    }

    to.extend(moved);
    dedup_preserve_order(to);
    group_categories(to);
}

pub fn normalize_category(name: &str) -> Option<String> {
    let parts: Vec<&str> = name
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

pub fn category_matches(category: Option<&str>, filter: &str) -> bool {
    let Some(category) = category else {
        return false;
    };
    category == filter || category.starts_with(&format!("{filter}/"))
}

pub fn category_badge(category: &str, filter: Option<&str>) -> Option<String> {
    match filter {
        None => Some(category.replace('/', "|")),
        Some(active) if category_matches(Some(category), active) => {
            let rel = category.strip_prefix(active).unwrap_or(category);
            let rel = rel.strip_prefix('/').unwrap_or(rel);
            if rel.is_empty() {
                None
            } else {
                Some(rel.replace('/', "|"))
            }
        }
        Some(_) => Some(category.replace('/', "|")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_category_filters_match_branch() {
        assert!(category_matches(Some("Work/work2"), "Work"));
        assert!(category_matches(Some("Work/work2"), "Work/work2"));
        assert!(!category_matches(Some("Personal/p1"), "Work"));
        assert!(!category_matches(None, "Work"));
    }

    #[test]
    fn category_badges_shorten_inside_active_branch() {
        assert_eq!(
            category_badge("Work/work2", None),
            Some("Work|work2".to_string())
        );
        assert_eq!(
            category_badge("Work/work2", Some("Work")),
            Some("work2".to_string())
        );
        assert_eq!(category_badge("Work", Some("Work")), None);
    }

    #[test]
    fn category_entries_include_parent_and_children() {
        let mut data = TodoData::new();
        data.add_category("Work/work1");
        data.add_category("Work/work2");

        let entries = data.category_entries();
        let rows: Vec<(&str, &str, usize, bool)> = entries
            .iter()
            .map(|entry| {
                (
                    entry.path.as_str(),
                    entry.label.as_str(),
                    entry.depth,
                    entry.is_all,
                )
            })
            .collect();

        assert_eq!(
            rows,
            vec![
                ("Work", "Work", 0, false),
                ("Work/work1", "work1", 1, false),
                ("Work/work2", "work2", 1, false),
            ]
        );
    }

    #[test]
    fn category_entries_group_children_added_after_another_root() {
        let mut data = TodoData::new();
        data.add_category("TRACE/Experiments");
        data.add_category("Personal/errands");
        data.add_category("TRACE/Vitrine");

        let rows: Vec<(&str, usize)> = data
            .category_entries()
            .iter()
            .map(|entry| (entry.label.as_str(), entry.depth))
            .collect();

        assert_eq!(
            rows,
            vec![
                ("TRACE", 0),
                ("Experiments", 1),
                ("Vitrine", 1),
                ("Personal", 0),
                ("errands", 1),
            ]
        );
    }

    #[test]
    fn notes_append_in_order_with_a_creation_date() {
        let mut data = TodoData::new();
        let id = data.add("call for a rdv");

        assert!(data.add_note(id, "no answer, retry morning"));
        assert!(data.add_note(id, "  rdv 15/09 10h30  "));

        let notes = &data.get(id).unwrap().notes;
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].text, "no answer, retry morning");
        assert_eq!(notes[1].text, "rdv 15/09 10h30");
        assert_eq!(notes[1].created, crate::date::Date::today().iso());
    }

    #[test]
    fn notes_reject_empty_and_oversized_text() {
        let mut data = TodoData::new();
        let id = data.add("item");

        assert!(!data.add_note(id, "   "));
        assert!(!data.add_note(id, &"é".repeat(NOTE_MAX_LEN + 1)));
        assert!(data.add_note(id, &"é".repeat(NOTE_MAX_LEN)));
        assert_eq!(data.note_count(id), 1);
    }

    #[test]
    fn notes_edit_and_delete_target_one_entry() {
        let mut data = TodoData::new();
        let id = data.add("item");
        data.add_note(id, "first");
        data.add_note(id, "second");

        assert!(data.update_note(id, 0, "first, edited"));
        assert!(!data.update_note(id, 9, "nowhere"));
        assert!(!data.update_note(id, 0, ""));

        let removed = data.delete_note(id, 0).unwrap();
        assert_eq!(removed.text, "first, edited");
        assert_eq!(data.note_count(id), 1);
        assert!(data.delete_note(id, 5).is_none());

        data.restore_note(id, 0, removed);
        assert_eq!(data.get(id).unwrap().notes[0].text, "first, edited");
        assert_eq!(data.get(id).unwrap().notes[1].text, "second");
    }

    #[test]
    fn notes_survive_a_delete_undo_round_trip() {
        let mut data = TodoData::new();
        let id = data.add("item");
        data.add_note(id, "kept through undo");

        let snapshot = data.delete(id).unwrap();
        data.restore_item(snapshot);

        assert_eq!(data.get(id).unwrap().notes[0].text, "kept through undo");
    }

    #[test]
    fn items_stored_without_notes_still_load() {
        let json = r#"{
            "next_id": 2,
            "items": [{
                "id": 1,
                "text": "legacy item",
                "done": false,
                "priority": "normal",
                "order": 0
            }]
        }"#;

        let data: TodoData = serde_json::from_str(json).expect("legacy payload should load");
        assert!(data.get(1).unwrap().notes.is_empty());
    }

    #[test]
    fn rename_category_updates_branch_items_and_stored_categories() {
        let mut data = TodoData::new();
        let id = data.add("ship");
        data.set_category(id, Some("Work/work1".to_string()));
        data.add_category("Work/work2");

        assert!(data.rename_category("Work", "Office"));

        let item = data.get(id).unwrap();
        assert_eq!(item.category.as_deref(), Some("Office/work1"));
        assert!(data.categories().contains(&"Office/work2".to_string()));
        assert!(!data.categories().contains(&"Work/work2".to_string()));
    }
}
