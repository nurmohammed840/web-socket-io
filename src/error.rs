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

