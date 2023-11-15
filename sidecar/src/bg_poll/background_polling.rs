use std::{sync::Arc, time::Duration};

use notify_debouncer_mini::{
    new_debouncer_opt,
    notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode},
    Config, DebounceEventResult, Debouncer,
};
use rand::{distributions, thread_rng, Rng};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::{
    application::application::Application,
    repo::types::{Backend, RepoRef, SyncStatus},
};

const POLL_INTERVAL_MINUTE: &[Duration] = &[
    Duration::from_secs(60),
    Duration::from_secs(3 * 60),
    Duration::from_secs(10 * 60),
    Duration::from_secs(20 * 60),
    Duration::from_secs(30 * 60),
];

/// Here we are going to watch for background changes to the file system, so we
/// can reindex the repository as and when it happens. This will allow us to
/// keep our index hot as we are making changes.
pub struct Poller {
    poll_interval_index: usize,
    minimum_interval_index: usize,
    git_events: flume::Receiver<()>,
    debouncer: Option<Debouncer<RecommendedWatcher>>,
}

impl Poller {
    fn start(app: &Application, reporef: &RepoRef) -> Option<Self> {
        let mut poll_interval_index = 0;
        let mut minimum_interval_index = 0;

        let (tx, rx) = flume::bounded(10);

        let mut _debouncer = None;
        if reporef.backend() == &Backend::Local {
            let disk_path = app.repo_pool.read(reporef, |_, v| v.disk_path.clone())?;

            let mut debouncer = debounced_events(tx);
            debouncer
                .watcher()
                .watch(&disk_path, RecursiveMode::Recursive)
                .map_err(|e| {
                    let d = disk_path.display();
                    error!(error = %e, path = %d, "path deleted");
                })
                .ok()?;
            _debouncer = Some(debouncer);

            info!(?reporef, ?disk_path, "reindexing as files have changed");

            poll_interval_index = POLL_INTERVAL_MINUTE.len() - 1;
            minimum_interval_index = POLL_INTERVAL_MINUTE.len() - 1;
        }

        Some(Self {
            poll_interval_index,
            minimum_interval_index,
            debouncer: _debouncer,
            git_events: rx,
        })
    }

    fn increase_interval(&mut self) -> Duration {
        self.poll_interval_index =
            (self.poll_interval_index + 1).min(POLL_INTERVAL_MINUTE.len() - 1);
        self.interval()
    }

    fn reset_interval(&mut self) -> Duration {
        self.poll_interval_index = self.minimum_interval_index;
        self.interval()
    }

    fn interval(&self) -> Duration {
        POLL_INTERVAL_MINUTE[self.poll_interval_index]
    }

    fn jittery_interval(&self) -> Duration {
        let poll_interval = self.interval();

        // add random jitter to avoid contention when jobs start at the same time
        let jitter = thread_rng().sample(distributions::Uniform::new(
            10,
            30 + poll_interval.as_secs() / 2,
        ));
        poll_interval + Duration::from_secs(jitter)
    }

    async fn git_change(&mut self) {
        if self.debouncer.is_some() {
            _ = self.git_events.recv_async().await;
            _ = self.git_events.drain().collect::<Vec<_>>();
        } else {
            loop {
                futures::pending!()
            }
        }
    }
}

fn check_repo(app: &Application, reporef: &RepoRef) -> Option<(i64, SyncStatus)> {
    app.repo_pool.read(reporef, |_, repo| {
        (repo.last_commit_unix_secs, repo.sync_status.clone())
    })
}

fn debounced_events(tx: flume::Sender<()>) -> Debouncer<RecommendedWatcher> {
    let notify_config: NotifyConfig = Default::default();

    let config = Config::default()
        .with_timeout(Duration::from_secs(5))
        .with_notify_config(notify_config.with_compare_contents(true));

    new_debouncer_opt(config, move |event: DebounceEventResult| match event {
        Ok(events) if !events.is_empty() => {
            if let Err(e) = tx.send(()) {
                error!("{e}");
            }
        }
        Ok(_) => debug!("no events received from debouncer"),
        Err(err) => {
            error!(?err, "repository monitoring");
        }
    })
    .expect("new_debouncer_opt to work")
}

// We only return Option<()> here so we can clean up a bunch of error
// handling code with `?`
//
// In reality this doesn't carry any meaning currently
async fn periodic_repo_poll(app: Application, reporef: RepoRef) -> Option<()> {
    debug!(?reporef, "repo monitoring started");
    let mut poller = Poller::start(&app, &reporef)?;

    loop {
        use SyncStatus::*;
        let (last_updated, status) = check_repo(&app, &reporef)?;
        if !status.indexable() {
            warn!(?status, "skipping indexing of repo");
            return None;
        }

        debug!("checking if sync in progress for re-indexing");
        if let Err(err) = app.write_index().block_until_synced(reporef.clone()).await {
            error!(?err, ?reporef, "sync failed");
            return None;
        }

        debug!("sync done for re-indexing");
        let (updated, status) = check_repo(&app, &reporef)?;
        if !status.indexable() {
            warn!(?status, ?reporef, "monitoring stopped");
            return None;
        }

        if last_updated == updated && status == Done {
            let poll_interval = poller.increase_interval();

            debug!(?reporef, ?poll_interval, "repo not changed, go figure")
        } else {
            let poll_interval = poller.reset_interval();

            debug!(
                ?reporef,
                ?last_updated,
                ?updated,
                ?poll_interval,
                "repo had updates"
            )
        }

        let timeout = tokio::time::sleep(poller.jittery_interval());
        tokio::select!(
            _ = timeout => {
                debug!(?reporef, "re-indexing");
                continue;
            },
            _ = poller.git_change() => {
                debug!(?reporef, "git changes triggered re-indexing");
                continue;
            }
        );
    }
}

pub async fn poll_repo_updates(app: Application) {
    let handles: Arc<scc::HashMap<RepoRef, JoinHandle<_>>> = Arc::default();
    loop {
        app.repo_pool
            .scan_async(|reporef, repo| match handles.entry(reporef.to_owned()) {
                scc::hash_map::Entry::Occupied(value) => {
                    if value.get().is_finished() {
                        _ = value.remove_entry();
                    }
                }
                scc::hash_map::Entry::Vacant(vacant) => {
                    if repo.sync_status.indexable() {
                        vacant.insert_entry(tokio::spawn(periodic_repo_poll(
                            app.clone(),
                            reporef.to_owned(),
                        )));
                    }
                }
            })
            .await;

        tokio::time::sleep(Duration::from_secs(5)).await
    }
}
