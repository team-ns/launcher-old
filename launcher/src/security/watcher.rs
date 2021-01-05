use anyhow::{Result};

use launcher_api::profile::Profile;
use notify::{Error, Event, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher as _};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};






enum Watcher {
    Recommended(RecommendedWatcher),
    Poll(PollWatcher),
}

pub struct WatcherService {
    watcher: Watcher,
    pub sender: Sender<Result<Event, Error>>,
    pub receiver: Receiver<Result<Event, Error>>,
}

impl WatcherService {
    pub fn new(profile: &Profile) -> Result<WatcherService> {
        let (sender, receiver) = mpsc::channel();

        let watcher = match create_watcher::<RecommendedWatcher>(profile, sender.clone()) {
            Ok(watcher) => Watcher::Recommended(watcher),
            Err(_) => Watcher::Poll(create_watcher::<PollWatcher>(profile, sender.clone())?),
        };

        Ok(WatcherService {
            watcher,
            sender,
            receiver,
        })
    }
}

fn create_watcher<T: notify::Watcher>(
    profile: &Profile,
    sender: Sender<Result<Event, Error>>,
) -> Result<T> {
    let mut watcher = T::new_immediate(move |res| {
        sender.send(res);
    })?;
    for path in &profile.update_verify {
        watcher.watch(path, RecursiveMode::Recursive)?;
    }
    for path in &profile.update_exclusion {
        watcher.unwatch(path)?;
    }
    Ok(watcher)
}
