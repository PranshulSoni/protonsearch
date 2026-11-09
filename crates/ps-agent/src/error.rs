use std::io;

/// Errors from the optional agent sidecar client.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    /// The Python venv interpreter or the sidecar entry point is missing.
    #[error("agent sidecar runtime is not available (missing python interpreter or entry point)")]
    NotAvailable,

    /// The child process could not be created or its pipes were not available.
    #[error("failed to spawn the agent sidecar: {0}")]
    Spawn(#[source] io::Error),

    /// The sidecar is not currently running (crashed, idle-killed or shut down).
    #[error("agent sidecar is not running")]
    NotRunning,

    /// A protocol frame would exceed the hard line bound.
    #[error("protocol frame is {actual} bytes; the limit is {limit} bytes")]
    LineTooLong { actual: usize, limit: usize },

    /// A message could not be encoded as JSON.
    #[error("failed to encode a protocol message: {0}")]
    Encode(#[from] serde_json::Error),

    /// Writing to or reading from the child failed.
    #[error("agent sidecar I/O failed: {0}")]
    Io(#[from] io::Error),
}

impl AgentError {
    pub(crate) fn spawn_io(message: &str) -> Self {
        AgentError::Spawn(io::Error::new(io::ErrorKind::Other, message.to_string()))
    }
}
