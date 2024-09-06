use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use once_cell::sync::Lazy;
use tempfile::TempDir;

pub enum DumpDir {
    TempDir(TempDir),
    SetDir(PathBuf),
}

impl DumpDir {
    pub fn path(&self) -> &Path {
        match self {
            DumpDir::TempDir(d) => d.path(),
            DumpDir::SetDir(d) => d.as_path(),
        }
    }
}

pub static WATCHER_LOGS_DIR: Lazy<DumpDir> =
    Lazy::new(|| match std::env::var_os("WATCHER_LOGS_DIR") {
        Some(logs_dir) => {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();

            let dump_dir = Path::new(&logs_dir).join(format!("watcher_{timestamp}"));
            std::fs::create_dir_all(&dump_dir).unwrap();

            return DumpDir::SetDir(dump_dir);
        }
        None => DumpDir::TempDir(TempDir::with_prefix("watcher").unwrap()),
    });
