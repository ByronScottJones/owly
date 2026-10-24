//! The frontend's input is untrusted (docs: tauri.md §5): paths and URLs are
//! checked here before anything is sent.

use std::path::PathBuf;

use crate::error::AppError;

const MAX_ITEMS: usize = 1000;
const MAX_URL_LEN: usize = 2048;

/// The items to send, as absolute paths of existing regular files or
/// directories, with symlinks resolved.
pub fn paths(items: &[String]) -> Result<Vec<PathBuf>, AppError> {
    if items.is_empty() || items.len() > MAX_ITEMS {
        return Err(AppError::Invalid(format!(
            "send between 1 and {MAX_ITEMS} items"
        )));
    }
    items
        .iter()
        .map(|item| {
            let path = PathBuf::from(item);
            if !path.is_absolute() {
                return Err(AppError::Invalid(format!(
                    "{item:?} isn't an absolute path"
                )));
            }
            let path = path
                .canonicalize()
                .map_err(|e| AppError::Invalid(format!("{item:?}: {e}")))?;
            let kind = path.metadata()?.file_type();
            if !(kind.is_file() || kind.is_dir()) {
                return Err(AppError::Invalid(format!(
                    "{item:?} is neither a file nor a folder"
                )));
            }
            Ok(path)
        })
        .collect()
}

/// The links to send: `http` or `https`, and no longer than a sane URL.
pub fn urls(items: &[String]) -> Result<Vec<String>, AppError> {
    if items.is_empty() || items.len() > MAX_ITEMS {
        return Err(AppError::Invalid(format!(
            "send between 1 and {MAX_ITEMS} links"
        )));
    }
    items
        .iter()
        .map(|item| {
            let url = url::Url::parse(item)
                .map_err(|e| AppError::Invalid(format!("{item:?} isn't a link: {e}")))?;
            if !matches!(url.scheme(), "http" | "https") || item.len() > MAX_URL_LEN {
                return Err(AppError::Invalid(format!(
                    "{item:?} isn't an http or https link of a sane length"
                )));
            }
            Ok(url.into())
        })
        .collect()
}

/// An interface name the kernel would accept, so a bad `OWLY_IFACE` is
/// refused at startup.
pub fn interface(name: &str) -> Result<(), AppError> {
    let ok = !name.is_empty()
        && name.len() <= 15
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    ok.then_some(())
        .ok_or_else(|| AppError::Invalid(format!("{name:?} isn't an interface name")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn relative_and_missing_paths_are_refused() {
        assert!(paths(&strings(&["photo.jpg"])).is_err());
        assert!(paths(&strings(&["/no/such/owly/file"])).is_err());
        assert!(paths(&[]).is_err());
    }

    #[test]
    fn an_existing_file_is_accepted_in_canonical_form() {
        let dir = std::env::temp_dir();
        let given = dir.join(".").to_string_lossy().into_owned();
        let got = paths(&[given]).expect("the temp dir exists");
        assert_eq!(got, [dir.canonicalize().expect("canonical")]);
    }

    #[test]
    fn only_web_links_are_accepted() {
        assert!(urls(&strings(&["https://example.org/a?b=c"])).is_ok());
        assert!(urls(&strings(&["file:///etc/passwd"])).is_err());
        assert!(urls(&strings(&["javascript:alert(1)"])).is_err());
        assert!(urls(&strings(&["not a url"])).is_err());
        assert!(urls(&[]).is_err());
    }

    #[test]
    fn interface_names_are_limited() {
        assert!(interface("awdl0").is_ok());
        assert!(interface("").is_err());
        assert!(interface("has space").is_err());
        assert!(interface("this-name-is-too-long").is_err());
    }
}
