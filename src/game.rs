use std::path::{PathBuf, Path};
use std::process::Command;
use std::env;
use jni::{InitArgsBuilder, JNIVersion, JavaVM};
use jni::djl::JvmLibrary;
use dlopen::wrapper::Container;

mod profile;

#[cfg(target_os = "windows")]
const EXPECTED_JVM_FILENAME: &str = "jvm.dll";
#[cfg(target_os = "linux")]
const EXPECTED_JVM_FILENAME: &str = "libjvm.so";
#[cfg(target_os = "macos")]
const EXPECTED_JVM_FILENAME: &str = "libjvm.dylib";

pub struct Client;

impl Client {
    pub fn start() {

        let profile = profile::Profile::new("profile.json").unwrap();

        let dir = &format!("{}libraries/", "/home/belz/.minecraft/");

        let java_home = match env::var("JAVA_HOME") {
            Ok(java_home) => PathBuf::from(java_home),
            Err(_) => find_java_home().expect(
                "Failed to find Java home directory. \
                 Try setting JAVA_HOME",
            ),
        };
        let libjvm_path = find_libjvm(java_home).unwrap();

        let args = InitArgsBuilder::new()
            .option(&profile.get_native_option())
            .option("-Dminecraft.client.jar=/home/belz/.minecraft/versions/Forge 1.7.10/Forge 1.7.10.jar")
            .option(&profile.create_lib_string(dir))
            .version(JNIVersion::V8)
            .build().unwrap();

        let lib: Container<JvmLibrary> = unsafe { Container::load(libjvm_path).unwrap() };


        let vm = JavaVM::new(args, lib).unwrap();
        let env = vm.attach_current_thread_permanently().unwrap();
        vm.attach_current_thread().unwrap().call_static_method("net/minecraft/launchwrapper/Launch", "main", "([Ljava/lang/String;)V", &[profile.create_args(&env)]).unwrap().v().unwrap();
    }
}

fn find_libjvm<S: AsRef<Path>>(path: S) -> Option<String> {
    let walker = walkdir::WalkDir::new(path).follow_links(true);

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_e) => continue,
        };

        let file_name = entry.file_name().to_str().unwrap_or("");

        if file_name == EXPECTED_JVM_FILENAME {
            return Some(entry.into_path().into_os_string().into_string().unwrap());
        }
    }
    None
}

fn find_java_home() -> Option<PathBuf> {
    Command::new("java")
        .arg("-XshowSettings:properties")
        .arg("-version")
        .output()
        .ok()
        .and_then(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            for line in stdout.lines().chain(stderr.lines()) {
                if line.contains("java.home") {
                    let pos = line.find('=').unwrap() + 1;
                    let path = line[pos..].trim();
                    return Some(PathBuf::from(path));
                }
            }
            None
        })
}