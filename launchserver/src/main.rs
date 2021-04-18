use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth::AuthProvider;
use crate::config::Config;
use crate::hash::HashingService;
use crate::profile::ProfileService;
use crate::security::SecurityService;
use std::ops::Deref;
use teloc::dev::container::{InstanceContainer, SingletonContainer};
use teloc::reexport::frunk::{HCons, HNil};
use teloc::{EmptyServiceProvider, ServiceProvider};

mod auth;
mod bundle;
mod commands;
mod config;
mod hash;
mod logger;
mod profile;
mod security;
mod server;

pub type LauncherServiceProvider = ServiceProvider<
    EmptyServiceProvider,
    HCons<
        InstanceContainer<Arc<RwLock<HashingService>>>,
        HCons<
            InstanceContainer<Arc<RwLock<ProfileService>>>,
            HCons<
                SingletonContainer<AuthProvider>,
                HCons<SingletonContainer<SecurityService>, HCons<SingletonContainer<Config>, HNil>>,
            >,
        >,
    >,
>;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    logger::configure();
    bundle::unpack_launcher();
    log::info!("Initialize launchserver components");
    let hash_service = Arc::new(RwLock::new(HashingService::new()));
    let profile_service = Arc::new(RwLock::new(ProfileService::new()));
    let sp: LauncherServiceProvider = teloc::ServiceProvider::new()
        .add_singleton::<Config>()
        .add_singleton::<SecurityService>()
        .add_singleton::<AuthProvider>()
        .add_instance(profile_service)
        .add_instance(hash_service);
    let sp_data = ntex::web::types::Data::new(sp);
    let sp_arc = sp_data.deref().clone();
    hash::rehash(sp_arc.clone(), &[]).await;
    log::info!("Start launchserver");
    tokio::join!(commands::run(sp_arc), server::run(sp_data))
        .1
        .expect("Can't run ntex server");
    Ok(())
}
