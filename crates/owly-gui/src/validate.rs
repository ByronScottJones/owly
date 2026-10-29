//! The frontend's input is untrusted (docs: tauri.md §5): paths and URLs are
//! checked here before anything is sent.

use std::path::PathBuf;

use crate::error::AppError;

const MAX_ITEMS: usize = 1000;
const MAX_URL_LEN: usize = 2048;

/// The items the user picked or dropped, as absolute paths of existing
/// regular files or directories, with symlinks resolved. The paths come
/// from the native picker or the window's drop event, never from the
/// frontend's own say-so.
pub fn existing(items: impl IntoIterator<Item = PathBuf>) -> Result<Vec<PathBuf>, AppError> {
    let items: Vec<PathBuf> = items.into_iter().collect();
    if items.is_empty() || items.len() > MAX_ITEMS {
        return Err(AppError::Invalid(format!(
            "send between 1 and {MAX_ITEMS} items"
        )));
    }
    items
        .into_iter()
        .map(|item| {
            let path = item
                .canonicalize()
                .map_err(|e| AppError::Invalid(format!("{}: {e}", item.display())))?;
            let kind = path.metadata()?.file_type();
            if !(kind.is_file() || kind.is_dir()) {
                return Err(AppError::Invalid(format!(
                    "{} is neither a file nor a folder",
                    item.display()
                )));
            }
            Ok(path)
        })
        .collect()
}

/// An on/off setting from the environment: `1` or `true`, `0` or `false`.
pub fn flag(name: &str, value: &str) -> Result<bool, AppError> {
    match value {
        "1" | "true" => Ok(true),
        "" | "0" | "false" => Ok(false),
        _ => Err(AppError::Invalid(format!("{name} must be 1 or 0"))),
    }
}

/// A directory for received files: absolute, so it doesn't depend on where
/// the app was started.
pub fn directory(path: PathBuf) -> Result<PathBuf, AppError> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(AppError::Invalid(format!(
            "{} isn't an absolute path",
            path.display()
        )))
    }
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
    #[test]
    fn a_flag_is_one_or_zero() {
        assert!(flag("X", "1").expect("on"));
        assert!(flag("X", "true").expect("on"));
        assert!(!flag("X", "0").expect("off"));
        assert!(!flag("X", "").expect("off"));
        assert!(flag("X", "yes").is_err());
    }

    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    // spec: OWLY-5
    #[test]
    fn missing_paths_are_refused() {
        assert!(existing([PathBuf::from("/no/such/owly/file")]).is_err());
        assert!(existing([]).is_err());
    }

    // spec: OWLY-5
    #[test]
    fn an_existing_directory_is_accepted_in_canonical_form() {
        let dir = std::env::temp_dir();
        let got = existing([dir.join(".")]).expect("the temp dir exists");
        assert_eq!(got, [dir.canonicalize().expect("canonical")]);
    }

    // spec: OWLY-5
    #[test]
    fn a_socket_is_neither_file_nor_folder() {
        let dir = std::env::temp_dir().join(format!("owly-gui-sock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temp dir");
        let path = dir.join("s");
        let _listener = std::os::unix::net::UnixListener::bind(&path).expect("a socket");
        let refused = existing([path]);
        std::fs::remove_dir_all(&dir).expect("cleanup");
        assert!(refused.is_err());
    }

    // spec: OWLY-5
    #[test]
    fn the_receive_directory_must_be_absolute() {
        assert!(directory(PathBuf::from("relative/dir")).is_err());
        assert!(directory(PathBuf::from("/abs/dir")).is_ok());
    }

    // spec: OWLY-5
    #[test]
    fn only_web_links_are_accepted() {
        assert!(urls(&strings(&["https://example.org/a?b=c"])).is_ok());
        assert!(urls(&strings(&["file:///etc/passwd"])).is_err());
        assert!(urls(&strings(&["javascript:alert(1)"])).is_err());
        assert!(urls(&strings(&["not a url"])).is_err());
        assert!(urls(&[]).is_err());
    }

    // spec: OWLY-5
    #[test]
    fn interface_names_are_limited() {
        assert!(interface("awdl0").is_ok());
        assert!(interface("").is_err());
        assert!(interface("has space").is_err());
        assert!(interface("this-name-is-too-long").is_err());
    }
}
