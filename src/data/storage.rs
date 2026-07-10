use std::path::PathBuf;

use super::TodoData;

pub fn data_path() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("todo-tui").join("todos.json")
}

pub struct Storage;

impl Storage {
    pub fn load() -> Result<TodoData, String> {
        let path = data_path();
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        serde_json::from_str(&raw).map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
    }

    pub fn backup_corrupt(reason: &str) -> Option<PathBuf> {
        let path = data_path();
        if !path.exists() {
            return None;
        }
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backup = path.with_extension(format!("json.corrupt-{ts}"));
        match std::fs::rename(&path, &backup) {
            Ok(()) => {
                eprintln!(
                    "todo-tui: could not load {} ({reason}); moved to {}",
                    path.display(),
                    backup.display()
                );
                Some(backup)
            }
            Err(e) => {
                eprintln!(
                    "todo-tui: could not load {} ({reason}); failed to back it up: {e}",
                    path.display()
                );
                None
            }
        }
    }

    pub fn save(data: &TodoData) -> Result<(), String> {
        let path = data_path();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
        }

        let raw = serde_json::to_string_pretty(data)
            .map_err(|e| format!("Failed to serialize: {}", e))?;

        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &raw)
            .map_err(|e| format!("Failed to write {}: {}", tmp_path.display(), e))?;

        std::fs::rename(&tmp_path, &path)
            .map_err(|e| format!("Failed to rename {}: {}", tmp_path.display(), e))?;

        Ok(())
    }
}
