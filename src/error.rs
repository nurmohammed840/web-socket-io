use std::fmt;

#[derive(Debug)]
pub struct ConnClose {
    pub code: u16,
    pub reason: Box<str>,
}

impl fmt::Display for ConnClose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl std::error::Error for ConnClose {}

#[derive(Debug)]
pub enum EmitError {
    EventNameTooBig,
    ReceiverClosed,
}

impl fmt::Display for EmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitError::EventNameTooBig => write!(f, "event name exceeds the allowed length."),
            EmitError::ReceiverClosed => write!(f, "receiver is already closed."),
        }
    }
}

impl std::error::Error for EmitError {}

#[derive(Debug)]
pub struct ReceiverClosed;

impl fmt::Display for ReceiverClosed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "receiver is already closed.")
    }
}
impl std::error::Error for ReceiverClosed {}
