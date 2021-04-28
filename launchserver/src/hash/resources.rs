use anyhow::{Context, Result};
use itertools::Itertools;
use launcher_api::message::ProfileResourcesResponse;
use launcher_api::optional::{Location, Optional, OptionalFiles};
use launcher_api::profile::{ProfileData, ProfileInfo};
use launcher_api::validation::{ClientInfo, OsType, RemoteDirectoryExt};
use std::collections::HashMap;

use crate::hash::HashingService;

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct NativeVersion {
    pub version: String,
    pub os_type: OsType,
}

impl NativeVersion {
    pub fn new(version: String, os_type: OsType) -> NativeVersion {
        NativeVersion { version, os_type }
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum FileLocation {
    Profile(String),
    Libraries(String),
    Assets(String),
    Natives(NativeVersion),
    Jres(OsType),
}

pub trait ProfileResources {
    fn get_resources(
        &self,
        client_info: &ClientInfo,
        profile_data: &ProfileData,
        optionals: &[String],
    ) -> Result<ProfileResourcesResponse>;
}

impl ProfileResources for HashingService {
    fn get_resources<'a>(
        &self,
        client_info: &'a ClientInfo,
        profile_data: &'a ProfileData,
        optionals: &'a [String],
    ) -> Result<ProfileResourcesResponse> {
        let selected_profile = &profile_data.profile;
        let profile_name = &selected_profile.name;
        let profile_info = &profile_data.profile_info;

        let rename_files = RenameFiles::new(profile_info, client_info, optionals);

        let profile = self
            .files
            .get(&FileLocation::Profile(profile_name.clone()))
            .context("Failed to find this profile!")?
            .clone()
            .filter_files(rename_files.get_files(Location::Profile));

        let assets = self
            .files
            .get(&FileLocation::Assets(selected_profile.assets.clone()))
            .context("Failed to find assets for this profile!")?
            .clone();

        let libraries = self
            .files
            .get(&FileLocation::Libraries(profile_name.clone()))
            .context("Failed to get libraries for this profile!")?
            .clone()
            .filter_files(rename_files.get_files(Location::Libraries));

        let natives = self
            .files
            .get(&FileLocation::Natives(NativeVersion::new(
                selected_profile.version.clone(),
                client_info.os_type.clone(),
            )))
            .context("Failed to find natives for this version and OsType")?
            .clone();

        let jre = self
            .files
            .get(&FileLocation::Jres(client_info.os_type.clone()))
            .context("Failed to find jre for this OsType")?
            .clone();

        Ok(ProfileResourcesResponse {
            profile,
            libraries,
            assets,
            natives,
            jre,
        })
    }
}

struct RenameFiles {
    irrelevant_files: HashMap<Location, Vec<OptionalFiles>>,
    relevant_files: HashMap<Location, Vec<OptionalFiles>>,
}

impl RenameFiles {
    fn new(
        profile_info: &ProfileInfo,
        client_info: &ClientInfo,
        optionals: &[String],
    ) -> RenameFiles {
        RenameFiles {
            irrelevant_files: Self::get_optional_files(
                profile_info.get_irrelevant_optionals(client_info, optionals),
            ),
            relevant_files: Self::get_optional_files(
                profile_info.get_relevant_optionals(client_info, optionals),
            ),
        }
    }

    fn get_optional_files<'a, I: Iterator<Item = &'a Optional>>(
        iter: I,
    ) -> HashMap<Location, Vec<OptionalFiles>> {
        iter.map(Optional::get_files).flatten().into_group_map()
    }

    fn get_files(
        &self,
        location: Location,
    ) -> (Option<&Vec<OptionalFiles>>, Option<&Vec<OptionalFiles>>) {
        (
            self.irrelevant_files.get(&location),
            self.relevant_files.get(&location),
        )
    }
}
