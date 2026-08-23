use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

const MAX_HISTORY_ENTRIES: usize = 1_000;

pub struct History {
    path: PathBuf,
    entries: Vec<String>,
}

impl History {
    pub fn load(path: PathBuf) -> io::Result<Self> {
        let entries = match fs::read_to_string(&path) {
            Ok(source) => source
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error),
        };
        let start = entries.len().saturating_sub(MAX_HISTORY_ENTRIES);
        Ok(Self {
            path,
            entries: entries[start..].to_vec(),
        })
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn resolve(&self, source: &str) -> Option<&str> {
        if source == "!!" {
            return self.entries.last().map(String::as_str);
        }
        source
            .strip_prefix('!')?
            .parse::<usize>()
            .ok()
            .and_then(|index| index.checked_sub(1))
            .and_then(|index| self.entries.get(index))
            .map(String::as_str)
    }

    pub fn push(&mut self, source: &str) -> io::Result<()> {
        if self.entries.last().is_some_and(|entry| entry == source) {
            return Ok(());
        }
        self.entries.push(source.to_owned());
        if self.entries.len() > MAX_HISTORY_ENTRIES {
            self.entries.remove(0);
        }
        save_atomic(&self.path, &self.entries)
    }

    pub fn clear(&mut self) -> io::Result<()> {
        self.entries.clear();
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

pub fn default_path() -> PathBuf {
    if let Some(path) = env::var_os("CALCITE_HISTORY").filter(|path| !path.is_empty()) {
        return PathBuf::from(path);
    }
    if let Some(directory) = env::var_os("XDG_STATE_HOME").filter(|path| !path.is_empty()) {
        return PathBuf::from(directory).join("calcite/history");
    }
    if let Some(home) = env::var_os("HOME").filter(|path| !path.is_empty()) {
        return PathBuf::from(home).join(".local/state/calcite/history");
    }
    env::temp_dir().join("calcite/history")
}

fn save_atomic(path: &Path, entries: &[String]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "history path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".history-{}.tmp", std::process::id()));
    let mut source = entries.join("\n");
    if !source.is_empty() {
        source.push('\n');
    }
    fs::write(&temporary, source)?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_resolves_and_clears_history() {
        let path = env::temp_dir().join(format!("calcite-history-test-{}", std::process::id()));
        let _ = fs::remove_file(&path);
        let mut history = History::load(path.clone()).unwrap();
        history.push("1 + 1").unwrap();
        history.push("2 + 2").unwrap();
        history.push("2 + 2").unwrap();
        assert_eq!(history.resolve("!!"), Some("2 + 2"));
        assert_eq!(history.resolve("!1"), Some("1 + 1"));
        assert_eq!(History::load(path.clone()).unwrap().entries().len(), 2);
        history.clear().unwrap();
        assert!(!path.exists());
    }
}
