use std::fmt;

/// Represents a connection closure with a code and reason.
#[derive(Debug)]
pub struct ConnClose {
    /// represents the status code of the close event.
    pub code: u16,
    /// represents the reason for the close event
    pub reason: Box<str>,
}

impl fmt::Display for ConnClose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl std::error::Error for ConnClose {}

/// Errors that can occur during notification.
#[derive(Debug)]
pub enum NotifyError {
    /// The event name exceeds the allowed size (255 bytes).
    EventNameTooBig,
    /// The receiver channel has been closed.
    ReceiverClosed,
}

impl fmt::Display for NotifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotifyError::EventNameTooBig => write!(f, "event name exceeds the allowed length."),
            NotifyError::ReceiverClosed => write!(f, "receiver is already closed."),
        }
    }
}

impl std::error::Error for NotifyError {}


/// Indicates that the receiver half is closed.
#[derive(Debug)]
pub struct ReceiverClosed;

impl fmt::Display for ReceiverClosed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "receiver is already closed.")
    }
}
impl std::error::Error for ReceiverClosed {}
