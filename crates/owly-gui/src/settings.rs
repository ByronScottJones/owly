//! The window's settings file (`~/.config/owly/owly_config.toml`, or under
//! `XDG_CONFIG_HOME`), which the preferences dialog edits. An environment
//! variable beats the file (docs: configuration.md, "Precedence"), and the
//! dialog then shows the setting as fixed.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::validate;

/// The file's name, in the `owly` folder of the user's config directory.
pub const FILE_NAME: &str = "owly_config.toml";
/// The environment variable that sets `[window] open_links` over the file.
pub const OPEN_LINKS_ENV: &str = "OWLY_OPEN_LINKS";

/// What the file holds. Unknown keys are refused, so a typo is found at
/// start-up instead of being ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Settings {
    pub window: WindowSettings,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WindowSettings {
    /// Whether a link received from another device may be opened in the
    /// browser, on a click (OWLY-7). Off unless the user turns it on.
    pub open_links: bool,
}

/// Which settings an environment variable fixes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Fixed {
    pub open_links: bool,
}

/// The settings in force, and where they came from.
#[derive(Debug, Clone, Default)]
pub struct Loaded {
    pub settings: Settings,
    pub fixed: Fixed,
}

/// What the page is told (src/model.rs `Status`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub open_links: bool,
    /// `OWLY_OPEN_LINKS` is set, so the dialog can't change it.
    pub open_links_fixed: bool,
    pub file: String,
}

/// `<config dir>/owly/owly_config.toml`.
pub fn path(config_dir: &Path) -> PathBuf {
    config_dir.join("owly").join(FILE_NAME)
}

/// The file's settings with the environment's laid over them. A missing
/// file is the defaults; a file that doesn't parse is an error naming it
/// (and the line), not something to start up past.
pub fn load(file: &Path, open_links_env: Option<&str>) -> Result<Loaded, AppError> {
    let mut settings = match std::fs::read_to_string(file) {
        Ok(text) => toml::from_str(&text)
            .map_err(|e| AppError::Invalid(format!("{}: {e}", file.display())))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
        Err(e) => return Err(AppError::Invalid(format!("{}: {e}", file.display()))),
    };
    let mut fixed = Fixed::default();
    if let Some(value) = open_links_env {
        settings.window.open_links = validate::flag(OPEN_LINKS_ENV, value)?;
        fixed.open_links = true;
    }
    Ok(Loaded { settings, fixed })
}

/// Writes the file whole, through a temporary file in the same folder so a
/// crash can't leave half of one. Mode `0600` in a `0700` folder: nobody
/// else may switch on what the user left off.
pub fn save(file: &Path, settings: &Settings) -> Result<(), AppError> {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

    let dir = file
        .parent()
        .ok_or_else(|| AppError::Invalid(format!("{} has no folder", file.display())))?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    let text = toml::to_string_pretty(settings)
        .map_err(|e| AppError::Invalid(format!("couldn't write the settings: {e}")))?;
    let temporary = dir.join(format!(".{FILE_NAME}.tmp"));
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)?;
    out.write_all(
        format!("# Written by the Owly window; edit here or in its preferences.\n{text}")
            .as_bytes(),
    )?;
    out.sync_all()?;
    std::fs::rename(&temporary, file)?;
    Ok(())
}

/// What turning `open_links` to `requested` needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// It is that already.
    Nothing,
    /// Turning it off: done at once.
    Apply,
    /// Turning it on: only once the user has said yes in a dialog the
    /// backend shows, which the page can't answer for them (OWLY-7).
    Confirm,
}

pub fn change_open_links(current: bool, fixed: Fixed, requested: bool) -> Result<Change, AppError> {
    if fixed.open_links {
        return Err(AppError::Unavailable(format!(
            "{OPEN_LINKS_ENV} sets this, so it can't be changed here"
        )));
    }
    Ok(match (current, requested) {
        (a, b) if a == b => Change::Nothing,
        (_, false) => Change::Apply,
        (_, true) => Change::Confirm,
    })
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn scratch() -> tempfile::TempDir {
        tempfile::tempdir().expect("a scratch directory")
    }

    // spec: OWLY-7
    #[test]
    fn links_are_off_with_no_file() {
        let dir = scratch();
        let loaded = load(&path(dir.path()), None).expect("defaults");
        assert!(!loaded.settings.window.open_links);
        assert!(!loaded.fixed.open_links);
    }

    // spec: OWLY-7
    #[test]
    fn what_is_saved_is_loaded_with_private_modes() {
        let dir = scratch();
        let file = path(dir.path());
        let on = Settings {
            window: WindowSettings { open_links: true },
        };
        save(&file, &on).expect("saves");
        assert_eq!(load(&file, None).expect("loads").settings, on);
        let mode = |p: &Path| std::fs::metadata(p).expect("exists").permissions().mode() & 0o777;
        assert_eq!(mode(&file), 0o600);
        assert_eq!(mode(file.parent().expect("a folder")), 0o700);
    }

    // spec: OWLY-7
    #[test]
    fn the_environment_beats_the_file_and_fixes_it() {
        let dir = scratch();
        let file = path(dir.path());
        save(
            &file,
            &Settings {
                window: WindowSettings { open_links: true },
            },
        )
        .expect("saves");
        let loaded = load(&file, Some("0")).expect("loads");
        assert!(!loaded.settings.window.open_links);
        assert!(loaded.fixed.open_links);
        assert!(load(&file, Some("maybe")).is_err());
    }

    #[test]
    fn a_bad_file_names_itself_and_is_not_ignored() {
        let dir = scratch();
        let file = path(dir.path());
        std::fs::create_dir_all(file.parent().expect("a folder")).expect("folder");
        for bad in ["[window]\nopen_link = true\n", "[window]\nopen_links = 3\n"] {
            std::fs::write(&file, bad).expect("writes");
            let error = load(&file, None).expect_err("refused").to_string();
            assert!(error.contains(FILE_NAME), "{error}");
        }
    }

    // spec: OWLY-7
    #[test]
    fn turning_links_on_needs_a_confirmation_and_off_does_not() {
        let free = Fixed::default();
        assert_eq!(
            change_open_links(false, free, true).expect("ok"),
            Change::Confirm
        );
        assert_eq!(
            change_open_links(true, free, false).expect("ok"),
            Change::Apply
        );
        assert_eq!(
            change_open_links(true, free, true).expect("ok"),
            Change::Nothing
        );
        let fixed = Fixed { open_links: true };
        assert!(change_open_links(false, fixed, true).is_err());
        assert!(change_open_links(true, fixed, false).is_err());
    }
}
