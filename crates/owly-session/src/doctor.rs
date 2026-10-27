//! The capability report that `owly doctor` and the GUI's Doctor panel show.

use owly_radio::probe;
use owly_radio::radio::Missing;
use owly_radio::rundir::{RUN_DIR, RunDir};
use owlyd::api;
use serde_json::Value;

/// The report, from owlyd, or from here with a word about owlyd when it
/// can't be reached.
pub async fn report(bus: Option<&str>) -> Value {
    let from_owlyd = async {
        let conn = api::connect(bus).await?;
        api::radio(&conn).await?.probe().await
    };
    match from_owlyd.await {
        Ok(json) => serde_json::from_str(&json).unwrap_or(Value::Null),
        Err(e) => {
            let mut caps = probe::from_config(RunDir::new(RUN_DIR));
            caps.missing.insert(
                0,
                Missing {
                    id: "backend",
                    say: format!(
                        "Owly's radio service, owlyd, can't be reached ({e}), so this report is from here. `systemctl enable --now owlyd` starts it."
                    ),
                },
            );
            serde_json::to_value(&caps).expect("capabilities serialize")
        }
    }
}
