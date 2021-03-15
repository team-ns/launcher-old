use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use path_slash::PathBufExt;
use walkdir::{DirEntry, WalkDir};

pub fn strip_folder(path: &Path, save_number: usize, skip_number: usize) -> String {
    path.iter()
        .take(save_number)
        .chain(path.iter().skip(save_number + skip_number))
        .collect::<PathBuf>()
        .to_slash_lossy()
}

pub fn get_files_from_dir<P: AsRef<Path>>(path: P) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(path)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.metadata().map(|m| m.is_file()).unwrap_or(false))
}

pub fn get_first_level_dirs<P: AsRef<Path>>(path: P) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.metadata().map(|m| m.is_dir()).unwrap_or(false))
}

pub fn strip(path: &Path, prefix: &str) -> Result<String> {
    Ok(path
        .strip_prefix(prefix)?
        .to_str()
        .with_context(|| {
            format!(
                "Can't strip prefix for path {:?}, maybe it is have non unicode chars!",
                path
            )
        })?
        .to_string())
}
