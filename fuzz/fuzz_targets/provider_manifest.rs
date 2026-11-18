#![no_main]
use libc_abi_tools::provider::Provider;
use libc_abi_tools::provider::{build::read_interfaces, json};
use libfuzzer_sys::fuzz_target;
fuzz_target!(|bytes: &[u8]| {
    if let Ok(text) = std::str::from_utf8(bytes) {
        if let Ok(value) = json::parse(text) {
            assert_eq!(json::parse(&value.to_text()).unwrap(), value);
        }
        let _ = Provider::parse(text);
        let _ = read_interfaces(text);
    }
});
