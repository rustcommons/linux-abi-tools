#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|bytes: &[u8]| {
    if let Ok(text) = std::str::from_utf8(bytes) {
        let _ = linux_abi_tools_fuzz::kernel_data::parse(text);
        let _ = kernel_abi_tools::Runner::parse(text);
        let _ = linux_targets::KernelVersion::parse(text);
    }
});
