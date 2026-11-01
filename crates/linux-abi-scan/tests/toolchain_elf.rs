//! Real acceptance fixtures are supplied externally, never bundled or executed.
use linux_abi_scan::{elf::Elf, scan, ScanTarget};

#[test]
#[ignore = "requires official x86_64 Rust toolchain ELF fixtures"]
fn accepts_official_cargo_and_rustc_driver() {
    let selection = linux_targets::Selection::resolve(&linux_targets::Request {
        glibc: Some("2.17"),
        kernel: Some("3.10"),
        ..Default::default()
    })
    .unwrap();
    for variable in [
        "LINUX_ABI_SCAN_CARGO_ELF",
        "LINUX_ABI_SCAN_RUSTC_DRIVER_ELF",
    ] {
        let path = std::env::var_os(variable).expect("fixture path must be supplied");
        let bytes = std::fs::read(&path).unwrap();
        let elf = Elf::parse(&bytes).unwrap_or_else(|e| panic!("{variable}: {e}"));
        assert!(
            !elf.imports.is_empty(),
            "fixture must exercise dynamic symbols"
        );
        let report = scan(&bytes, &ScanTarget::from_selection(&selection))
            .unwrap_or_else(|e| panic!("{variable}: {e}"));
        assert!(!report.findings.is_empty());
        eprintln!(
            "{variable}: {} bytes accepted, {} findings",
            bytes.len(),
            report.findings.len()
        );
    }
}
