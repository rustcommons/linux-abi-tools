#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|bytes: &[u8]| {
    if let Ok(text) = std::str::from_utf8(bytes) {
        if let Ok(version) = text.parse::<libc_abi_tools::version::Version>() {
            assert_eq!(
                version
                    .to_string()
                    .parse::<libc_abi_tools::version::Version>()
                    .unwrap(),
                version
            );
            let _ = version.validate_release();
        }
        let _ = linux_targets::split_versioned_target(text);
        for request in [
            linux_targets::Request {
                target: Some(text),
                ..Default::default()
            },
            linux_targets::Request {
                glibc: Some(text),
                ..Default::default()
            },
            linux_targets::Request {
                kernel: Some(text),
                ..Default::default()
            },
            linux_targets::Request {
                distro: Some(text),
                ..Default::default()
            },
        ] {
            let _ = linux_targets::Selection::resolve(&request);
        }
    }
});
