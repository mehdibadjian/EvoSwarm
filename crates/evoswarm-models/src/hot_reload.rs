use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
use tokio::sync::mpsc;

use crate::config::{load, ModelConfig};
use crate::error::ConfigError;

/// Atomically swappable handle to the active model configuration.
///
/// `current()` returns an owned `Arc` snapshot, so a concurrent reload can never
/// tear a read: an in-flight call keeps the config instance it started with.
#[derive(Clone)]
pub struct ConfigHandle {
    inner: Arc<ArcSwap<ModelConfig>>,
}

impl ConfigHandle {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let config = load(path)?;
        Ok(Self {
            inner: Arc::new(ArcSwap::from_pointee(config)),
        })
    }

    pub fn current(&self) -> Arc<ModelConfig> {
        self.inner.load_full()
    }

    /// Re-reads and validates `path`; on success swaps in the new config and
    /// returns the new snapshot, on failure leaves the active config untouched.
    pub fn reload(&self, path: &Path) -> Result<Arc<ModelConfig>, ConfigError> {
        let config = load(path)?;
        let next = Arc::new(config);
        self.inner.store(Arc::clone(&next));
        Ok(next)
    }

    /// True when `snapshot` is the exact instance currently active. Used to
    /// assert that a reload swapped (or deliberately did not swap) the config.
    pub fn same_instance(&self, snapshot: &Arc<ModelConfig>) -> bool {
        Arc::ptr_eq(&self.inner.load_full(), snapshot)
    }
}

/// Serves reload triggers until `signals` closes. Each trigger re-reads `path`;
/// a parse/validate failure is reported on stderr and leaves the active config
/// in place so the daemon stays healthy.
///
/// Production wires `tokio::signal::unix::SignalKind::hangup()` into `signals`.
pub async fn serve_reload_signals(
    handle: ConfigHandle,
    path: PathBuf,
    mut signals: mpsc::Receiver<()>,
) {
    while signals.recv().await.is_some() {
        match handle.reload(&path) {
            Ok(_) => eprintln!("config reloaded from {}", path.display()),
            Err(err) => eprintln!("config reload rejected, keeping previous config: {err}"),
        }
    }
}

/// Bridges SIGHUP to a reload channel. Spawned once at daemon startup; returns
/// when the process is shutting down or the signal stream ends.
#[cfg(unix)]
pub async fn forward_sighup(tx: mpsc::Sender<()>) {
    use tokio::signal::unix::{signal, SignalKind};

    let mut stream = match signal(SignalKind::hangup()) {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("unable to register SIGHUP handler: {err}");
            return;
        }
    };
    while stream.recv().await.is_some() {
        if tx.send(()).await.is_err() {
            break;
        }
    }
}
