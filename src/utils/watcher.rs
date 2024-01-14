use std::path::PathBuf;

use log::error;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

type Event = notify::Result<notify::Event>;

#[derive(Debug)]
pub struct FileChangeWatcher {
    paths_to_watch: Vec<PathBuf>,
    _watcher: RecommendedWatcher,
    events_rx: std::sync::mpsc::Receiver<Event>,
}

impl FileChangeWatcher {
    pub fn new(files: &[&str]) -> Self {
        let paths_to_watch: Vec<PathBuf> = files.iter().map(|s| s.parse().unwrap()).collect();
        let (events_tx, events_rx) = std::sync::mpsc::channel::<Event>();
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = events_tx.send(res);
            },
            notify::Config::default(),
        )
        .expect("could not create Watcher");

        for path in paths_to_watch.iter() {
            _ = watcher.watch(path, RecursiveMode::NonRecursive);
        }

        FileChangeWatcher {
            paths_to_watch,
            _watcher: watcher,
            events_rx,
        }
    }

    pub fn check_for_changes(&self) -> Option<Vec<&PathBuf>> {
        let mut result: Vec<&PathBuf> = vec![];
        while let Ok(event) = self.events_rx.try_recv() {
            if let Ok(event) = event {
                if let notify::Event {
                    kind: EventKind::Modify(_),
                    paths,
                    attrs: _,
                } = event
                {
                    for p in paths {
                        for q in self.paths_to_watch.iter() {
                            // necessary because the paths in `paths_to_watch` are relative and the paths in the event are absolute.
                            let path_equals = p
                                .as_path()
                                .to_str()
                                .expect("Path should be utf8")
                                .ends_with(q.to_str().expect("Path should be utf8"));
                            if path_equals {
                                result.push(q);
                            }
                        }
                    }
                }
            }
        }
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }
}

/// Watches a wgsl file and can be polled for changes in this file by [`ShaderFileWatcher::check_for_changes`].
/// Only valid wgsl is returned as a change. If invalid, you are not notified. But still wgsl can cause panics if not lining up with your pipeline.
#[derive(Debug)]
pub struct ShaderFileWatcher {
    wgsl_file: PathBuf,
    watcher: FileChangeWatcher,
}

impl ShaderFileWatcher {
    pub fn new(path: &str) -> Self {
        let wgsl_file: PathBuf = path.parse().expect("invalid path");
        if !wgsl_file.exists() {
            error!("Wgsl file at {wgsl_file:?} path not found!");
        }
        Self {
            wgsl_file,
            watcher: FileChangeWatcher::new(&[path]),
        }
    }
}

impl ShaderFileWatcher {
    /// Returns the new wgsl content of the file in case a change was detected
    pub fn check_for_changes(&self) -> Option<String> {
        if let Some(e) = self.watcher.check_for_changes() {
            if !e.contains(&&self.wgsl_file) {
                return None;
            }
            let wgsl = std::fs::read_to_string(&self.wgsl_file).unwrap();
            if wgsl.trim().is_empty() {
                // Note: There is currently some bug that causes empty strings to be returned here. Need to be hunted down.
                // But not so important to figure out right now.
                return None;
            }
            if let Err(err) = wgpu::naga::front::wgsl::parse_str(&wgsl) {
                error!("WGSL at {:?} is invalid: {err}", self.wgsl_file);
            } else {
                println!("Hot reloaded WGSL from {:?}", self.wgsl_file);

                return Some(wgsl);
            }
        }
        None
    }
}

// pub struct FileWatcher {
//     commands_tx: mpsc::UnboundedSender<WatchCommand>,
//     events_rx: mpsc::UnboundedReceiver<Event>,
//     current_events: Vec<Event>,
// }

// enum WatchCommand {
//     /// Watch a path
//     Watch { path: PathBuf, recursive: bool },
// }

// impl FileWatcher {
//     /// # Async
//     ///
//     /// Expects a tokio runtime in the background.
//     pub fn new() -> FileWatcher {
//         let (commands_tx, events_rx) = spawn_watch_task();
//         FileWatcher {
//             commands_tx,
//             events_rx,
//             current_events: vec![],
//         }
//     }

//     pub fn watch(&self, path: &Path) {
//         let path: PathBuf = path.into();
//         self.commands_tx
//             .send(WatchCommand::Watch {
//                 path,
//                 recursive: false,
//             })
//             .unwrap();
//     }

//     pub fn file_modified(&self, path: &Path) -> bool {
//         self.current_events.iter().any(|e| {
//             if let Ok(notify::Event {
//                 kind: EventKind::Modify(ModifyKind::Data(_)),
//                 paths,
//                 attrs: _,
//             }) = e
//             {
//                 paths.iter().any(|e| {
//                     let is_same = e
//                         .as_path()
//                         .to_str()
//                         .expect("Path should be utf8")
//                         .ends_with(path.to_str().expect("Path should be utf8"));
//                     is_same
//                 })
//             } else {
//                 false
//             }
//         })
//     }

//     pub fn update(&mut self) {
//         self.current_events.clear();
//         while let Ok(event) = self.events_rx.try_recv() {
//             self.current_events.push(event);
//         }
//     }
// }

// fn spawn_watch_task() -> (
//     mpsc::UnboundedSender<WatchCommand>,
//     mpsc::UnboundedReceiver<Event>,
// ) {
//     let (commands_tx, mut commands_rx) = mpsc::unbounded_channel::<WatchCommand>();
//     let (events_tx, events_rx) = mpsc::unbounded_channel::<Event>();
//     let events_tx_2 = events_tx.clone();

//     let mut watcher = RecommendedWatcher::new(
//         move |res| {
//             let _ = events_tx.send(res);
//         },
//         notify::Config::default(),
//     )
//     .expect("could not create Watcher");

//     // let watcher = Arc::new(Mutex::new(Watch));

//     tokio::spawn(async move {
//         while let Some(command) = commands_rx.recv().await {
//             match command {
//                 WatchCommand::Watch { path, recursive } => {
//                     let recursive = if recursive {
//                         RecursiveMode::Recursive
//                     } else {
//                         RecursiveMode::NonRecursive
//                     };
//                     if let Err(err) = watcher.watch(&path, recursive) {
//                         events_tx_2
//                             .send(Err(err))
//                             .expect("Could not send watcher errror");
//                     }
//                 }
//             }
//         }
//     });

//     (commands_tx, events_rx)
// }
