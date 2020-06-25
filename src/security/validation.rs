use crate::client::downloader;
use crate::client::downloader::RemoteFile;
use anyhow::anyhow;
use anyhow::Result;
use launcher_api::validation::{HashedFile, HashedProfile};
use std::path::PathBuf;

pub enum ValidationStatus {
    Success,
    NeedUpdate(Vec<RemoteFile>),
}

pub async fn validate_profile(
    game_dir: String,
    profile_name: String,
    profile: HashedProfile,
    file_server: String,
) -> Result<()> {
    let profile_path = format!("{}/{}/", game_dir, profile_name);
    let exists = PathBuf::from(&profile_path)
        .read_dir()
        .map(|mut dir| dir.next().is_some())
        .unwrap_or(false);

    if !exists {
        let remote = profile_into_remote(profile.clone().into_iter());
        downloader::download(remote, file_server.clone()).await?;
    }
    validate_then_download(profile.clone(), game_dir.clone(), file_server).await?;
    match validate(profile, game_dir)? {
        ValidationStatus::Success => Ok(()),
        ValidationStatus::NeedUpdate(files) => Err(anyhow!("Sync error: {:?}", files)),
    }
}

fn validate(profile: HashedProfile, game_dir: String) -> Result<ValidationStatus> {
    let profile = profile
        .into_iter()
        .filter(|file| HashedFile::new(&format!("{}/{}", game_dir, file.0)) == file.1);
    let remote = profile_into_remote(profile);
    if remote.is_empty() {
        Ok((ValidationStatus::Success))
    } else {
        Ok((ValidationStatus::NeedUpdate(remote)))
    }
}

async fn validate_then_download(
    profile: HashedProfile,
    game_dir: String,
    file_server: String,
) -> Result<()> {
    match validate(profile, game_dir)? {
        ValidationStatus::NeedUpdate(files) => downloader::download(files, file_server).await?,
        _ => {}
    };
    Ok(())
}

fn profile_into_remote(iter: impl Iterator<Item = (String, HashedFile)>) -> Vec<RemoteFile> {
    iter.map(|file| RemoteFile {
        name: file.0,
        size: file.1.size,
    })
    .collect()
}
