use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use thiserror::Error;
use tokio_util::sync::CancellationToken;

const POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("failed to spawn curl: {0}")]
    Spawn(std::io::Error),

    #[error("curl exited with {code}; stderr: {stderr}")]
    Failed { code: i32, stderr: String },

    #[error("curl timed out after {timeout_ms}ms")]
    TimedOut { timeout_ms: u64 },

    #[error("curl was cancelled")]
    Cancelled,

    #[error("failed to wait for curl: {0}")]
    Wait(std::io::Error),

    #[error("failed to create curl capture file: {0}")]
    CaptureFile(std::io::Error),

    #[error("failed to read curl capture file: {0}")]
    CaptureRead(std::io::Error),

    #[error("response is not UTF-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
}

pub fn get_text(
    url: &str,
    timeout: Option<Duration>,
    cancellation: &CancellationToken,
) -> Result<String, DownloadError> {
    let bytes = get_bytes(url, timeout, cancellation)?;
    String::from_utf8(bytes).map_err(DownloadError::InvalidUtf8)
}

pub fn get_bytes(
    url: &str,
    timeout: Option<Duration>,
    cancellation: &CancellationToken,
) -> Result<Vec<u8>, DownloadError> {
    if timeout.is_some_and(|value| value.is_zero()) {
        return Err(DownloadError::TimedOut { timeout_ms: 0 });
    }
    let stdout = tempfile::NamedTempFile::new().map_err(DownloadError::CaptureFile)?;
    let stderr = tempfile::NamedTempFile::new().map_err(DownloadError::CaptureFile)?;
    let started = Instant::now();
    let mut child = Command::new("curl")
        .args([
            "-fsSL",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: v8-runner",
            url,
        ])
        .stdout(Stdio::from(
            stdout.reopen().map_err(DownloadError::CaptureFile)?,
        ))
        .stderr(Stdio::from(
            stderr.reopen().map_err(DownloadError::CaptureFile)?,
        ))
        .spawn()
        .map_err(DownloadError::Spawn)?;

    loop {
        if let Some(status) = child.try_wait().map_err(DownloadError::Wait)? {
            if !status.success() {
                return Err(DownloadError::Failed {
                    code: status.code().unwrap_or(-1),
                    stderr: read_capture_string(stderr.path())?,
                });
            }
            return std::fs::read(stdout.path()).map_err(DownloadError::CaptureRead);
        }

        if cancellation.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(DownloadError::Cancelled);
        }

        if let Some(limit) = timeout {
            let elapsed = started.elapsed();
            if elapsed >= limit {
                let _ = child.kill();
                let _ = child.wait();
                return Err(DownloadError::TimedOut {
                    timeout_ms: limit.as_millis() as u64,
                });
            }
            std::thread::sleep(POLL_INTERVAL.min(limit.saturating_sub(elapsed)));
        } else {
            std::thread::sleep(POLL_INTERVAL);
        }
    }
}

fn read_capture_string(path: &std::path::Path) -> Result<String, DownloadError> {
    let bytes = std::fs::read(path).map_err(DownloadError::CaptureRead)?;
    Ok(String::from_utf8_lossy(&bytes).trim().to_owned())
}
