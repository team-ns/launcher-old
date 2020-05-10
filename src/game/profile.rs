use jni::objects::{JObject, JValue};
use jni::JNIEnv;
use launcher_api::profile::Profile;
use path_slash::PathExt;
use std::fs::File;
use std::path::{Path, PathBuf};

#[cfg(not(target_os = "windows"))]
const CLASS_PATH_SEPARATOR: &str = ":";
#[cfg(target_os = "windows")]
const CLASS_PATH_SEPARATOR: &str = ";";

pub trait ClientProfile {
    fn new(path: &str) -> Self;
    fn create_lib_string(&self, dir: &str) -> String;
    fn get_native_option(&self, dir: &str) -> String;
    fn create_args(&self, dir: &str, env: &JNIEnv) -> JValue;
    fn get_client_dir(&self, dir: &str) -> PathBuf;
}

pub fn check_profile(path: &str) {
    //TODO add file check
}

impl ClientProfile for Profile {
    fn new(path: &str) -> Profile {
        println!("{}", path);
        serde_json::from_reader(File::open(path).unwrap()).unwrap()
    }

    fn create_lib_string(&self, dir: &str) -> String {
        let mut path = String::from("-Djava.class.path=");
        for library in &self.libraries {
            path += &Path::new(&[dir, "/libraries/", library, CLASS_PATH_SEPARATOR].join(""))
                .to_slash_lossy();
        }
        let class_path: Vec<_> = self
            .class_path
            .iter()
            .map(|s| self.get_client_dir(dir).join(&s).to_slash_lossy())
            .collect();
        path += &class_path.join(CLASS_PATH_SEPARATOR);
        println!("{}", path);
        path
    }

    fn get_native_option(&self, dir: &str) -> String {
        format!(
            "{}{}",
            "-Djava.library.path=",
            Path::new(dir)
                .join("natives")
                .join(&self.version)
                .to_slash_lossy()
        )
    }

    fn create_args(&self, dir: &str, env: &JNIEnv) -> JValue {
        let mut args = self.client_args.clone();
        args.push(String::from("--gameDir"));
        args.push(self.get_client_dir(dir).to_string_lossy().to_string());
        args.push(String::from("--assetsDir"));
        args.push(
            Path::new(dir)
                .join(&self.assets_dir)
                .to_slash_lossy()
                .to_string(),
        );
        args.push(String::from("--assetIndex"));
        args.push(self.assets.to_string());

        let array = env
            .new_object_array(
                args.len() as i32,
                env.find_class("java/lang/String").unwrap(),
                JObject::from(env.new_string("").unwrap()),
            )
            .unwrap();
        for i in 0..args.len() {
            env.set_object_array_element(
                array,
                i as i32,
                JObject::from(env.new_string(&args[i]).unwrap()),
            )
            .unwrap();
        }
        JValue::from(JObject::from(array))
    }

    fn get_client_dir(&self, dir: &str) -> PathBuf {
        Path::new(dir).join(&self.name)
    }
}
