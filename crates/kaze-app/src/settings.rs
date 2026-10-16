//! What Kaze remembers between launches, as JSON in the config directory.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Folders holding the user's projects, scanned for worktrees.
    pub project_folders: Vec<PathBuf>,
}

fn file() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("Kaze").join("settings.json"))
}

impl Settings {
    pub fn load() -> Self {
        file().map(|f| Self::load_from(&f)).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(f) = file() {
            let _ = self.save_to(&f);
        }
    }

    /// Missing or unreadable file: defaults (a broken file must not stop the app).
    fn load_from(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Adds folders, skipping ones already there.
    pub fn add_folders(&mut self, folders: impl IntoIterator<Item = PathBuf>) {
        for folder in folders {
            if !self.project_folders.contains(&folder) {
                self.project_folders.push(folder);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("settings.json");
        let mut s = Settings::default();
        s.add_folders([
            PathBuf::from("/a"),
            PathBuf::from("/b"),
            PathBuf::from("/a"),
        ]);
        assert_eq!(s.project_folders.len(), 2, "no duplicates");
        s.save_to(&path).unwrap();
        assert_eq!(Settings::load_from(&path), s);
    }

    #[test]
    fn broken_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(Settings::load_from(&path), Settings::default());
        assert_eq!(
            Settings::load_from(&dir.path().join("missing")),
            Settings::default()
        );
    }
}
