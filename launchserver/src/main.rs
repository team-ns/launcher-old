use std::ops::Deref;
use std::sync::Arc;

use teloc::dev::container::{InstanceContainer, SingletonContainer};
use teloc::reexport::frunk::{HCons, HNil};
use teloc::{EmptyServiceProvider, Resolver, ServiceProvider};
use tokio::sync::RwLock;

use crate::auth::AuthProvider;
use crate::config::Config;
use crate::extensions::ExtensionService;
use crate::hash::HashingService;
use crate::profile::ProfileService;
use crate::security::SecurityService;

mod auth;
mod build;
mod commands;
mod config;
mod extensions;
mod hash;
mod logger;
mod profile;
mod security;
mod server;
mod util;

pub type LauncherServiceProvider = ServiceProvider<
    EmptyServiceProvider,
    HCons<
        InstanceContainer<Arc<RwLock<HashingService>>>,
        HCons<
            InstanceContainer<Arc<RwLock<ProfileService>>>,
            HCons<
                SingletonContainer<ExtensionService>,
                HCons<
                    SingletonContainer<AuthProvider>,
                    HCons<
                        SingletonContainer<SecurityService>,
                        HCons<SingletonContainer<Config>, HNil>,
                    >,
                >,
            >,
        >,
    >,
>;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    logger::configure();
    build::bundle::unpack_launcher();
    log::info!("Initialize launchserver components");
    let hash_service = Arc::new(RwLock::new(HashingService::new()));
    let profile_service = Arc::new(RwLock::new(ProfileService::new()));
    let sp: LauncherServiceProvider = teloc::ServiceProvider::new()
        .add_singleton::<Config>()
        .add_singleton::<SecurityService>()
        .add_singleton::<AuthProvider>()
        .add_singleton::<ExtensionService>()
        .add_instance(profile_service)
        .add_instance(hash_service);
    let sp_data = ntex::web::types::Data::new(sp);
    let sp_arc = sp_data.deref().clone();
    let _: &SecurityService = sp_arc.resolve();
    let _: &AuthProvider = sp_arc.resolve();
    hash::rehash(sp_arc.clone(), &[]).await;
    log::info!("Start launchserver");
    let extension_service: &ExtensionService = sp_arc.resolve();
    extension_service
        .initialize_extensions()
        .expect("Can't initialize extensions");
    let result = tokio::try_join!(commands::run(sp_arc), server::run(sp_data));
    if let Err(e) = result {
        log::error!("Can't run launchserver with error: {}", e)
    }
    Ok(())
}
