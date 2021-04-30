use anyhow::{Context, Result};
use path_slash::PathBufExt;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

pub fn strip_folder(path: &Path, save_number: usize, skip_number: usize) -> String {
    path.iter()
        .take(save_number)
        .chain(path.iter().skip(save_number + skip_number))
        .collect::<PathBuf>()
        .to_slash_lossy()
}

fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.depth() == 0 || !s.starts_with("."))
        .unwrap_or(false)
}

pub fn get_files_from_dir<P: AsRef<Path>>(path: P) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(path)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| is_not_hidden(e))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
}

pub fn get_first_level_dirs<P: AsRef<Path>>(path: P) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
}

pub fn strip(path: &Path, prefix: &str) -> Result<String> {
    Ok(path
        .strip_prefix(prefix)?
        .to_str()
        .context(format!(
            "Can't strip prefix for path {:?}, maybe it is have non unicode chars!",
            path
        ))?
        .to_string())
}
