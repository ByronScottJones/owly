#![no_main]

use std::time::Instant;

use libfuzzer_sys::fuzz_target;
use owly_airdrop::proto::mdns::{Browser, Message, Service, MAX_MESSAGE};

fuzz_target!(|data: &[u8]| {
    let Ok(message) = Message::decode(data) else { return };
    // What decodes re-encodes and decodes the same, unless encoding without
    // compression outgrows the limit decode enforces.
    let encoded = message.encode();
    if encoded.len() <= MAX_MESSAGE {
        assert_eq!(Message::decode(&encoded).unwrap(), message);
    }
    let now = Instant::now();
    let mut browser = Browser::new(now);
    browser.handle(&message, now);
    while browser.poll_event().is_some() {}
    let service = Service {
        instance: "3fa9c01b22de".into(),
        host: "owly".into(),
        port: 8770,
        addr: "fe80::1".parse().unwrap(),
        flags: 0x89,
    };
    let _ = service.answer(&message, data.len() % 2 == 0);
});
