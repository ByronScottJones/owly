use std::sync::Arc;

/// Where a session's passing remarks go: stderr for the CLI, a log or an
/// event for a GUI.
#[derive(Clone)]
pub struct Notes(Arc<dyn Fn(&str) + Send + Sync>);

impl Notes {
    pub fn new(sink: impl Fn(&str) + Send + Sync + 'static) -> Self {
        Notes(Arc::new(sink))
    }

    pub fn stderr() -> Self {
        Notes::new(|line| eprintln!("{line}"))
    }

    pub fn say(&self, line: impl AsRef<str>) {
        (self.0)(line.as_ref());
    }
}

impl std::fmt::Debug for Notes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Notes")
    }
}
