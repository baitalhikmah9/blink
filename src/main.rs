//! Blink! for macOS: menu-bar blink cue + 20-20-20 reminder.

mod animation;
mod app;
mod autostart;
mod break_banner;
mod idle;
mod overlay;
mod scheduler;
mod settings;
mod tray;

use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

const LOCK_FILE: &str = "Library/Application Support/Blink/lock";

// macOS fcntl.h values for advisory open-time locking.
const O_NONBLOCK: i32 = 0x00000004;
const O_EXLOCK: i32 = 0x00000020;

fn lock_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(LOCK_FILE))
}

/// Acquire a per-user lock file using macOS O_EXLOCK.
/// `ErrorKind::WouldBlock` means another instance is running.
fn try_lock(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .custom_flags(O_NONBLOCK | O_EXLOCK)
        .open(path)
}

fn ensure_single_instance() -> std::io::Result<std::fs::File> {
    let Some(path) = lock_path() else {
        return Err(std::io::Error::other("HOME not set"));
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    try_lock(&path)
}

fn main() {
    let _lock = match ensure_single_instance() {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            eprintln!("Blink! is already running.");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Blink! lock error: {e}");
            std::process::exit(1);
        }
    };
    app::run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique_lock_path() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("blink-lock-test-{n}"))
    }

    #[test]
    fn lock_granted_and_released() {
        let tmp = unique_lock_path();
        let _ = std::fs::remove_file(&tmp);
        let f = try_lock(&tmp).expect("first lock should succeed");

        let second = try_lock(&tmp);
        assert_eq!(second.unwrap_err().kind(), std::io::ErrorKind::WouldBlock);

        drop(f);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn different_paths_lock_independently() {
        let a = unique_lock_path();
        let b = unique_lock_path();
        let _ = std::fs::remove_file(&a);
        let _ = std::fs::remove_file(&b);

        let f_a = try_lock(&a).expect("lock a");
        let f_b = try_lock(&b).expect("lock b");

        drop(f_a);
        drop(f_b);
        let _ = std::fs::remove_file(&a);
        let _ = std::fs::remove_file(&b);
    }
}
