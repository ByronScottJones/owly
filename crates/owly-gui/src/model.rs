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

/// The longest name, file name or link the window shows, in characters.
const MAX_SHOWN: usize = 255;
const MAX_LINK_SHOWN: usize = 2048;
/// How many files or links of one offer are listed; the rest are counted.
pub const MAX_LISTED: usize = 100;

/// Text from the other end of the link, made fit to show (SEC-4): control
/// characters become U+FFFD, the characters that reorder or hide text
/// (bidirectional overrides, zero-width marks) are dropped so a name can't
/// pass for another, and the length is bounded.
pub fn display_text(text: &str, max: usize) -> String {
    let mut shown = String::new();
    for (i, c) in text.chars().filter(|c| !hides_text(*c)).enumerate() {
        if i == max {
            shown.push('…');
            break;
        }
        shown.push(if c.is_control() { '\u{FFFD}' } else { c });
    }
    shown
}

fn hides_text(c: char) -> bool {
    matches!(
        c,
        '\u{061C}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
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
            name: display_text(&l.name, MAX_SHOWN),
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
    Files {
        files: Vec<OfferedFile>,
        more: usize,
    },
    Urls {
        urls: Vec<String>,
        more: usize,
    },
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
                    .take(MAX_LISTED)
                    .map(|f| OfferedFile {
                        name: display_text(&f.name, MAX_SHOWN),
                        kind: display_text(&f.file_type.to_string(), MAX_SHOWN),
                        size: f.size,
                    })
                    .collect(),
                more: files.len().saturating_sub(MAX_LISTED),
            },
            Offer::Urls(urls) => IncomingOffer::Urls {
                urls: urls
                    .iter()
                    .take(MAX_LISTED)
                    .map(|u| display_text(u, MAX_LINK_SHOWN))
                    .collect(),
                more: urls.len().saturating_sub(MAX_LISTED),
            },
        };
        IncomingRequest {
            id,
            sender_name: display_text(&ask.sender_computer_name, MAX_SHOWN),
            sender_model: display_text(&ask.sender_model_name, MAX_SHOWN),
            offer,
        }
    }
}

/// What the user picked or dropped, as the backend holds it. The frontend
/// only ever names it by `id`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionInfo {
    pub id: u64,
    pub count: usize,
    pub names: Vec<String>,
    pub more: usize,
}

/// Files dropped on the window, and where.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dropped {
    pub selection: SelectionInfo,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct DragPoint {
    pub x: f64,
    pub y: f64,
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
    fn controls_are_replaced_and_reordering_dropped() {
        assert_eq!(display_text("a\nb\u{7}", 20), "a\u{FFFD}b\u{FFFD}");
        // A right-to-left override would make "evil\u{202E}gpj.exe" read as "evilexe.jpg".
        assert_eq!(display_text("evil\u{202E}gpj.exe", 20), "evilgpj.exe");
        assert_eq!(display_text("a\u{200B}b\u{2066}c", 20), "abc");
    }

    #[test]
    fn long_text_is_cut_with_an_ellipsis() {
        assert_eq!(display_text("abcdef", 3), "abc…");
        assert_eq!(display_text("abc", 3), "abc");
        assert_eq!(display_text(&"é".repeat(1000), 255).chars().count(), 256);
    }

    #[test]
    fn an_offer_is_bounded_and_counted() {
        use owly_airdrop::proto::messages::AskFile;
        let files: Vec<AskFile> = (0..MAX_LISTED + 5)
            .map(|i| AskFile {
                name: format!("f{i}\u{202E}"),
                file_type: "public.data".into(),
                size: Some(1),
                bom_path: String::new(),
                is_directory: false,
            })
            .collect();
        let ask = AskRequest {
            transfer_id: None,
            transfer_type: None,
            sender_computer_name: "Ph\u{202E}one".into(),
            sender_model_name: "m".into(),
            sender_id: String::new(),
            bundle_id: String::new(),
            sender_record: None,
            file_icon: None,
            offer: Offer::Files(files),
            described: String::new(),
        };
        let shown = IncomingRequest::new(1, &ask);
        assert_eq!(shown.sender_name, "Phone");
        let IncomingOffer::Files { files, more } = shown.offer else {
            panic!("a file offer stays one");
        };
        assert_eq!(files.len(), MAX_LISTED);
        assert_eq!(more, 5);
        assert_eq!(files[0].name, "f0");
    }

    #[test]
    fn visibility_starts_off() {
        assert_eq!(Visibility::default(), Visibility::Off);
    }
}
