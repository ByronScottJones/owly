//! AirDrop on an owly link, without a user interface: finding receivers,
//! sending and receiving, as `owly-cli` and `owly-gui` each present them.
//!
//! The functions here report progress through [`Notes`] rather than
//! printing, so each front end shows it its own way.

pub mod bluez;
pub mod discover;
pub mod doctor;
mod ids;
pub mod link;
mod notes;
pub mod radio;
pub mod receive;
pub mod wifi;

pub use ids::{AIRDROP_PORT, RECEIVE_PORT, new_sender, parse_mac, random_service_id};
pub use notes::Notes;
