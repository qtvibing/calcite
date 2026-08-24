use std::{env, fs, io, path::PathBuf};

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

pub fn ensure_parent(path: &std::path::Path) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "history path has no parent"))?;
    fs::create_dir_all(parent)
}
