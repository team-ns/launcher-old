use jni::objects::{JObject, JString};
use jni::JNIEnv;

use once_cell::sync::Lazy;
use std::str::FromStr;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use uuid::Uuid;
pub type AuthGetChannel = Lazy<(
    Arc<Mutex<Sender<(String, Uuid, String)>>>,
    Arc<Mutex<Receiver<(String, Uuid, String)>>>,
)>;

pub static CHANNEL_GET: AuthGetChannel = Lazy::new(|| {
    let (rx, tx) = std::sync::mpsc::channel();
    (Arc::new(Mutex::new(rx)), Arc::new(Mutex::new(tx)))
});

pub type AuthSendChannel = Lazy<(Arc<Mutex<Sender<String>>>, Arc<Mutex<Receiver<String>>>)>;
pub static CHANNEL_SEND: AuthSendChannel = Lazy::new(|| {
    let (rx, tx) = std::sync::mpsc::channel();
    (Arc::new(Mutex::new(rx)), Arc::new(Mutex::new(tx)))
});

#[no_mangle]
#[allow(non_snake_case)]
pub(crate) extern "system" fn Java_com_mojang_authlib_yggdrasil_launcherJoinRequest(
    env: JNIEnv,
    _object: JObject,
    request: JObject,
) {
    struct JNI<'a>(JNIEnv<'a>, JObject<'a>);
    impl<'a> JNI<'a> {
        fn get_field(&self, name: &'a str, ty: &'a str) -> Result<JObject<'a>, jni::errors::Error> {
            self.0.get_field(self.1, name, ty).and_then(|v| v.l())
        }
        fn get_string(&self, result: Result<JObject, jni::errors::Error>) -> String {
            result
                .map(JString::from)
                .and_then(|jstr| self.0.get_string(jstr))
                .unwrap()
                .into()
        }
        fn to_string(
            &self,
            result: Result<JObject<'a>, jni::errors::Error>,
        ) -> Result<JObject<'a>, jni::errors::Error> {
            result
                .and_then(|obj| {
                    self.0
                        .call_method(obj, "toString", "()Ljava/lang/String;", &[])
                })
                .and_then(|v| v.l())
        }
    }

    let jni = JNI(env, request);
    let profile: Uuid = {
        let result = jni.to_string(jni.get_field("selectedProfile", "Ljava/util/UUID;"));
        Uuid::from_str(&jni.get_string(result)).expect("Can't parse game profile unique id")
    };
    let server: String = { jni.get_string(jni.get_field("serverId", "Ljava/lang/String;")) };
    let token: String = { jni.get_string(jni.get_field("accessToken", "Ljava/lang/String;")) };
    CHANNEL_GET
        .0
        .lock()
        .unwrap()
        .send((token, profile, server))
        .expect("Can't send join info to client");
    let string = CHANNEL_SEND.1.lock().unwrap().recv().unwrap();
    if !string.is_empty() {
        env.throw_new(
            "com/mojang/authlib/exceptions/AuthenticationException",
            string,
        )
        .expect("Can't throw auth exception");
    }
}
