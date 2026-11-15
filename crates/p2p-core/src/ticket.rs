use std::{fmt, str::FromStr};

use crate::Error;

const PREFIX: &str = "p2p1";

/// What the sender gives the receiver (as text or QR code) to connect.
///
/// The payload will carry the sender's node address and the content hash.
/// For now it is an opaque string, so the format can evolve behind this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    payload: String,
}

impl Ticket {
    pub fn new(payload: impl Into<String>) -> Self {
        Self {
            payload: payload.into(),
        }
    }

    pub fn payload(&self) -> &str {
        &self.payload
    }
}

impl fmt::Display for Ticket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{PREFIX}{}", self.payload)
    }
}

impl FromStr for Ticket {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        match s.strip_prefix(PREFIX) {
            Some(payload) if !payload.is_empty() => Ok(Self::new(payload)),
            _ => Err(Error::InvalidTicket(s.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_text() {
        let ticket = Ticket::new("abc123");
        let parsed: Ticket = ticket.to_string().parse().unwrap();
        assert_eq!(parsed, ticket);
    }

    #[test]
    fn rejects_unknown_prefix() {
        assert!("xyz".parse::<Ticket>().is_err());
        assert!("p2p1".parse::<Ticket>().is_err());
    }
}
