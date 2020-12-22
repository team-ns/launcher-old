use anyhow::Result;
use dlopen::wrapper::Container;
use jni::djl::JvmLibrary;
use jni::{InitArgsBuilder, JNIVersion, JavaVM, NativeMethod};
use launcher_api::profile::Profile;
use profile::ClientProfile;
use std::env;
use std::path::{Path, PathBuf};

use crate::client::AuthInfo;
use crate::game::auth::{Java_com_mojang_authlib_yggdrasil_launcherJoinRequest, Java_example};
use std::os::raw::c_void;
use tokio::runtime::Handle;

pub(crate) mod auth;
mod profile;

#[cfg(target_os = "windows")]
const JVM_LIB_PATH: &str = "bin/server/jvm.dll";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const JVM_LIB_PATH: &str = "lib/amd64/server/libjvm.so";
#[cfg(all(target_os = "linux", target_arch = "x86"))]
const JVM_LIB_PATH: &str = "lib/i386/server/libjvm.so";
#[cfg(target_os = "macos")]
const JVM_LIB_PATH: &str = "lib/server/libjvm.dylib";

pub fn create_jvm(profile: Profile, dir: &str) -> Result<JavaVM> {
    let args = InitArgsBuilder::new()
        .option("-Dfml.ignoreInvalidMinecraftCertificates=true")
        .option("-Dfml.ignorePatchDiscrepancies=true")
        .option("-XX:+DisableAttachMechanism")
        .option(&profile.get_native_option(dir))
        .option(&profile.create_lib_string(dir))
        .version(JNIVersion::V8)
        .build();

    if cfg!(windows) {
        let mut bin_path = env::current_dir()?;
        bin_path.push("jre/bin");
        match env::var_os("PATH") {
            Some(path) => {
                let mut paths = env::split_paths(&path).collect::<Vec<_>>();
                paths.push(PathBuf::from(bin_path));
                let new_path = env::join_paths(paths)?;
                env::set_var("PATH", &new_path);
            }
            None => env::set_var("PATH", bin_path),
        }
    }

    match args {
        Ok(args) => {
            let lib: Container<JvmLibrary> = unsafe { Container::load(JVM_LIB_PATH)? };
            Ok(JavaVM::new(args, lib)?)
        }
        Err(_) => Err(anyhow::anyhow!("Failed to create java args!")),
    }
}

pub fn start(jvm: JavaVM, profile: Profile, auth_info: AuthInfo, dir: &str) -> Result<()> {
    let jni_env = jvm.attach_current_thread_permanently()?;
    let method = NativeMethod {
        name: "launcherJoinRequest".into(),
        sig: "(Lcom/mojang/authlib/yggdrasil/request/JoinMinecraftServerRequest;)V".into(),
        fn_ptr: Java_com_mojang_authlib_yggdrasil_launcherJoinRequest as *mut c_void,
    };
    jni_env.register_native_methods(
        "com/mojang/authlib/yggdrasil/YggdrasilMinecraftSessionService",
        &vec![method],
    );
    env::set_current_dir(profile.get_client_dir(dir))?;
    jni_env
        .call_static_method(
            &profile.main_class,
            "main",
            "([Ljava/lang/String;)V",
            &[profile.create_args(dir, &jni_env, auth_info)],
        )?
        .v()?;
    Ok(())
}
