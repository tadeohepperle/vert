use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub mod timing_queue;
pub use timing_queue::{EntryKey, Timing, TimingQueue};
pub mod watcher;

/// Returns the file location of a .wgsl file with the same name as the .rs file, this was invoked in.
#[macro_export]
macro_rules! wgsl_file {
    () => {{
        // drop the rs, add wgsl
        let mut wgsl_file = format!("./{}", file!()).replace("./framework", ".");
        // replace because we want it to not be from the workspace parent folder.
        wgsl_file.pop();
        wgsl_file.pop();
        wgsl_file.push_str("wgsl");
        wgsl_file
    }};
}
