#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|bytes: &[u8]| {
    let _ = libc_abi_tools::elf::Metadata::parse(bytes);
    let _ = linux_abi_scan::elf::Elf::parse(bytes);
    let selection = linux_targets::Selection::resolve(&linux_targets::Request {
        glibc: Some("2.17"),
        kernel: Some("3.10"),
        ..Default::default()
    })
    .unwrap();
    let target = linux_abi_scan::ScanTarget::from_selection(&selection);
    let _ = linux_abi_scan::scan(bytes, &target);
});
