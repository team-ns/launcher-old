use anyhow::Result;
use dlopen::wrapper::Container;
use jni::djl::JvmLibrary;
use jni::{InitArgsBuilder, JNIVersion, JavaVM};
use launcher_api::profile::Profile;
use path_slash::PathBufExt;
use profile::ClientProfile;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
//use tokio::sync::Mutex;

use crate::client::AuthInfo;

mod profile;

#[cfg(target_os = "windows")]
const EXPECTED_JVM_FILENAME: &str = "jvm.dll";
#[cfg(target_os = "linux")]
const EXPECTED_JVM_FILENAME: &str = "libjvm.so";
#[cfg(target_os = "macos")]
const EXPECTED_JVM_FILENAME: &str = "libjvm.dylib";

pub fn create_jvm(profile: Profile, dir: &str) -> Result<JavaVM> {
    let java_home = match env::var("JAVA_HOME") {
        Ok(java_home) => PathBuf::from(java_home),
        Err(_) => find_java_home().expect(
            "Failed to find Java home directory. \
                 Try setting JAVA_HOME",
        ),
    };
    let libjvm_path = find_libjvm(java_home).unwrap();

    let args = InitArgsBuilder::new()
        .option("-Dfml.ignoreInvalidMinecraftCertificates=true")
        .option("-Dfml.ignorePatchDiscrepancies=true")
        .option("-XX:+DisableAttachMechanism")
        .option(&profile.get_native_option(dir))
        .option(&profile.create_lib_string(dir))
        .version(JNIVersion::V8)
        .build();
    if args.is_ok() {
        let lib: Container<JvmLibrary> = unsafe { Container::load(libjvm_path)? };
        Ok(JavaVM::new(args.ok().unwrap(), lib)?)
    } else {
        Err(anyhow::anyhow!("Failed to create java args!"))
    }
}

pub fn start(jvm: JavaVM, profile: Profile, auth_info: AuthInfo, dir: &str) -> Result<()> {
    let env = jvm.attach_current_thread_permanently()?;
    env::set_current_dir(profile.get_client_dir(dir))?;
    env.call_static_method(
        &profile.main_class,
        "main",
        "([Ljava/lang/String;)V",
        &[profile.create_args(dir, &env, auth_info)],
    )
    .unwrap()
    .v()?;
    Ok(())
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
            return Some(entry.into_path().to_slash_lossy());
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
