use launcher_api::validation::{HashedProfile, HashedFile};
use anyhow::Error;
use crate::client::downloader::RemoteFile;

pub enum ValidationStatus {
    Success,
    NeedUpdate(Vec<RemoteFile>)
}

pub fn validate(profile: HashedProfile, game_dir: String) -> Result<ValidationStatus, Error> {
    let profile = profile
        .into_iter()
        .filter(|file| HashedFile::new(&format!("{}/{}", game_dir, file.0)) == file.1)
        .map(|file| RemoteFile { name: file.0, size: file.1.size})
        .collect::<Vec<_>>();
    if profile.is_empty() {
        Ok((ValidationStatus::Success))
    } else {
        Ok((ValidationStatus::NeedUpdate(profile)))
    }
}