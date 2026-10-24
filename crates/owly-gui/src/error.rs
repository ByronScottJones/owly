use owly_session::radio::{Failure, RadioError};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// What a command fails with. It crosses to the frontend as
/// `{ kind, message }`, so the UI can choose what to offer for each kind.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Radio(#[from] RadioError),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// The frontend sent something that isn't acceptable.
    #[error("{0}")]
    Invalid(String),
    /// A request that can't be met on this machine.
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    NotFound(String),
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Radio(e) => match e.kind {
                Failure::Unreachable => "radio_unreachable",
                Failure::NotAuthorized => "not_authorized",
                Failure::Busy => "radio_busy",
                Failure::NotNow => "radio_not_now",
                Failure::NoRadio => "no_radio",
                Failure::Other => "radio",
            },
            AppError::Io(_) => "io",
            AppError::Invalid(_) => "invalid",
            AppError::Unavailable(_) => "unavailable",
            AppError::NotFound(_) => "not_found",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("AppError", 2)?;
        out.serialize_field("kind", self.kind())?;
        out.serialize_field("message", &self.to_string())?;
        out.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_crosses_as_kind_and_message() {
        let e = AppError::Unavailable("needs an identity".into());
        let json = serde_json::to_value(&e).expect("serializes");
        assert_eq!(
            json,
            serde_json::json!({"kind": "unavailable", "message": "needs an identity"})
        );
    }

    #[test]
    fn a_radio_error_keeps_its_kind() {
        let e = AppError::from(RadioError {
            kind: Failure::NotAuthorized,
            message: "polkit said no".into(),
        });
        assert_eq!(e.kind(), "not_authorized");
    }
}
