use crate::optional::Optional;
use crate::validation::ClientInfo;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub version: String,
    pub libraries: Vec<String>,
    pub class_path: Vec<String>,
    pub main_class: String,
    pub update_verify: Vec<String>,
    pub update_exclusion: Vec<String>,
    pub jvm_args: Vec<String>,
    pub client_args: Vec<String>,
    pub assets: String,
    pub assets_dir: String,
    pub server_name: String,
    pub server_port: u32,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ProfileInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub optionals: Vec<Optional>,
}

impl ProfileInfo {
    pub fn retain_visible_optionals(&mut self, client_info: &ClientInfo) {
        self.optionals
            .retain(|optional| optional.visible(client_info))
    }
    pub fn get_relevant_optionals<'a>(
        &'a self,
        client_info: &'a ClientInfo,
        selected: &'a Vec<String>,
    ) -> impl Iterator<Item = &'a Optional> {
        self.get_optionals_by_filter(move |optional| optional.relevant(client_info, selected))
    }

    pub fn get_irrelevant_optionals<'a>(
        &'a self,
        client_info: &'a ClientInfo,
        selected: &'a Vec<String>,
    ) -> impl Iterator<Item = &'a Optional> {
        self.get_optionals_by_filter(move |optional| !optional.relevant(client_info, selected))
    }

    fn get_optionals_by_filter<P: FnMut(&&Optional) -> bool>(
        &self,
        predicate: P,
    ) -> impl Iterator<Item = &Optional> {
        self.optionals.iter().filter(predicate)
    }
}

pub struct ProfileData {
    pub profile: Profile,
    pub profile_info: ProfileInfo,
}
