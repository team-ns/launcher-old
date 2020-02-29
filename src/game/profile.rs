use std::fs::File;
use std::io::{Result, Error};
use serde::{Serialize, Deserialize};
use jni::objects::{JValue, JObject};
use jni::JNIEnv;

#[derive(Serialize, Deserialize)]
pub struct Profile {
    name: String,
    libraries: Vec<String>,
    main: String,
    assets: String,
}

impl Profile {
    pub fn new(path: &str) -> Result<Profile> {
        match serde_json::from_reader(File::open(path)?) {
            Ok(profile) => Ok(profile),
            Err(e) => Err(Error::from(e)),
        }
    }

    pub fn create_lib_string(&self, dir: &str) -> String {
        let mut path = "-Djava.class.path=".to_string();
        for library in &self.libraries {
            path += &[dir, library, ":"].join("");
        }
        path + &self.main
    }

    pub fn get_native_option(&self) -> String {
        ["-Djava.library.path=", &self.name, "/native"].join("")
    }

    pub fn create_args(&self, env: &JNIEnv) -> JValue {
        let vec = vec!["--username", "Belz", "--version", "1.7.10", "--accessToken", "0", "--userProperties", "{}", "--gameDir", "/home/belz/.minecraft/versions", "--assetsDir", "/home/belz/.minecraft/assets/", "--assetIndex", "1.7.10", "--tweakClass", "cpw.mods.fml.common.launcher.FMLTweaker"];
        let array= env.new_object_array(vec.len() as i32, env.find_class("java/lang/String").unwrap(), JObject::from(env.new_string("").unwrap())).unwrap();
        for i in 0..vec.len() {
            env.set_object_array_element(array, i as i32, JObject::from(env.new_string(vec[i]).unwrap())).unwrap();
        }
        JValue::from(JObject::from(array))
    }
}