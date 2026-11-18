#![no_main]
use libc_abi_tools::{
    glibc::{parse_abilist, Snapshot},
    matrix::Matrix,
};
use libfuzzer_sys::fuzz_target;
fuzz_target!(|bytes: &[u8]| {
    if let Ok(text) = std::str::from_utf8(bytes) {
        if let Ok(symbols) = parse_abilist(text) {
            let snapshot = Snapshot {
                release: "2.17".parse().unwrap(),
                libraries: [("c".into(), symbols)].into(),
            };
            let _ = snapshot.to_tsv(None);
            let _ = Matrix::from_snapshots([snapshot]);
        }
    }
});
