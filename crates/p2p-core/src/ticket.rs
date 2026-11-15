use std::{fmt, str::FromStr};

use iroh_blobs::{ticket::BlobTicket, BlobFormat};

use crate::Error;

/// What the sender gives the receiver (as text or QR code) so it can connect and download.
///
/// It contains the sender's public key, its known addresses and the hash of the shared content.
/// It contains no secret: anyone holding it can download the share while the sender is online.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket(pub(crate) BlobTicket);

impl fmt::Display for Ticket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Ticket {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let inner: BlobTicket = s
            .parse()
            .map_err(|e| Error::InvalidTicket(format!("{e}")))?;
        if inner.format() != BlobFormat::HashSeq {
            return Err(Error::InvalidTicket("not a p2p share".into()));
        }
        Ok(Self(inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_garbage() {
        assert!("xyz".parse::<Ticket>().is_err());
        assert!("".parse::<Ticket>().is_err());
    }
}
