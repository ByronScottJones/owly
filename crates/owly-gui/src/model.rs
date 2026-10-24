//! What crosses the IPC boundary, as the frontend's `api.ts` declares it.

use owly_airdrop::proto::messages::{AskRequest, Offer};
use owly_session::discover::Listed;
use serde::{Deserialize, Serialize};

/// Who can see this computer (docs/SPEC.md, ID-6).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Everyone,
    ContactsOnly,
    #[default]
    Off,
}

/// A receiver nearby. `id` is all the frontend may name one by.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    pub id: String,
    pub name: String,
}

impl From<&Listed> for Peer {
    fn from(l: &Listed) -> Self {
        Peer {
            id: l.instance.clone(),
            name: l.name.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub name: String,
    pub interface: String,
    pub visibility: Visibility,
    pub radio: RadioStatus,
    pub receive_dir: String,
    /// Whether Contacts Only can be chosen: not without an Apple identity,
    /// which owly-airdrop doesn't implement (docs/implementation.md).
    pub contacts_only_available: bool,
    pub browsing: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RadioStatus {
    /// owlyd's state: `idle`, `starting`, `up` or `stopping`; `unavailable`
    /// when owlyd can't be asked.
    pub state: String,
    /// Whether this window holds the lease.
    pub held: bool,
}

/// One file in an offer. All of it is the sender's say-so (SEC-4).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferedFile {
    pub name: String,
    pub kind: String,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IncomingOffer {
    Files { files: Vec<OfferedFile> },
    Urls { urls: Vec<String> },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingRequest {
    pub id: u64,
    pub sender_name: String,
    pub sender_model: String,
    pub offer: IncomingOffer,
}

impl IncomingRequest {
    pub fn new(id: u64, ask: &AskRequest) -> Self {
        let offer = match &ask.offer {
            Offer::Files(files) => IncomingOffer::Files {
                files: files
                    .iter()
                    .map(|f| OfferedFile {
                        name: f.name.clone(),
                        kind: f.file_type.to_string(),
                        size: f.size,
                    })
                    .collect(),
            },
            Offer::Urls(urls) => IncomingOffer::Urls { urls: urls.clone() },
        };
        IncomingRequest {
            id,
            sender_name: ask.sender_computer_name.clone(),
            sender_model: ask.sender_model_name.clone(),
            offer,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferState {
    Waiting,
    Accepted,
    Progress,
    Delivered,
    Declined,
    Failed,
}

/// Each step of a transfer we send.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferUpdate {
    pub id: u64,
    pub peer: String,
    pub state: TransferState,
    pub sent: u64,
    pub total: u64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Received {
    pub id: u64,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedUrls {
    pub id: u64,
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiveFailed {
    pub id: u64,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_uses_snake_case_names() {
        assert_eq!(
            serde_json::to_string(&Visibility::ContactsOnly).expect("serializes"),
            "\"contacts_only\""
        );
        let parsed: Visibility = serde_json::from_str("\"everyone\"").expect("parses");
        assert_eq!(parsed, Visibility::Everyone);
    }

    #[test]
    fn visibility_starts_off() {
        assert_eq!(Visibility::default(), Visibility::Off);
    }
}
