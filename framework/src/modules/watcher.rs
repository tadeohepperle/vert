use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use notify::{event::ModifyKind, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

type Event = notify::Result<notify::Event>;

pub struct FileWatcher {
    commands_tx: mpsc::UnboundedSender<WatchCommand>,
    events_rx: mpsc::UnboundedReceiver<Event>,
    current_events: Vec<Event>,
}

enum WatchCommand {
    /// Watch a path
    Watch { path: PathBuf, recursive: bool },
}

impl FileWatcher {
    /// # Async
    ///
    /// Expects a tokio runtime in the background.
    pub fn new() -> FileWatcher {
        let (commands_tx, events_rx) = spawn_watch_task();
        FileWatcher {
            commands_tx,
            events_rx,
            current_events: vec![],
        }
    }

    pub fn watch(&self, path: &Path) {
        let path: PathBuf = path.into();
        self.commands_tx
            .send(WatchCommand::Watch {
                path,
                recursive: false,
            })
            .unwrap();
    }

    pub fn file_modified(&self, path: &Path) -> bool {
        self.current_events.iter().any(|e| {
            if let Ok(notify::Event {
                kind: EventKind::Modify(ModifyKind::Data(_)),
                paths,
                attrs: _,
            }) = e
            {
                paths.iter().any(|e| {
                    let is_same = e
                        .as_path()
                        .to_str()
                        .expect("Path should be utf8")
                        .ends_with(path.to_str().expect("Path should be utf8"));
                    is_same
                })
            } else {
                false
            }
        })
    }

    pub fn update(&mut self) {
        self.current_events.clear();
        while let Ok(event) = self.events_rx.try_recv() {
            self.current_events.push(event);
        }
    }
}

fn spawn_watch_task() -> (
    mpsc::UnboundedSender<WatchCommand>,
    mpsc::UnboundedReceiver<Event>,
) {
    let (commands_tx, mut commands_rx) = mpsc::unbounded_channel::<WatchCommand>();
    let (events_tx, events_rx) = mpsc::unbounded_channel::<Event>();
    let events_tx_2 = events_tx.clone();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = events_tx.send(res);
        },
        notify::Config::default(),
    )
    .expect("could not create Watcher");

    // let watcher = Arc::new(Mutex::new(Watch));

    tokio::spawn(async move {
        while let Some(command) = commands_rx.recv().await {
            match command {
                WatchCommand::Watch { path, recursive } => {
                    let recursive = if recursive {
                        RecursiveMode::Recursive
                    } else {
                        RecursiveMode::NonRecursive
                    };
                    if let Err(err) = watcher.watch(&path, recursive) {
                        events_tx_2
                            .send(Err(err))
                            .expect("Could not send watcher errror");
                    }
                }
            }
        }
    });

    (commands_tx, events_rx)
}