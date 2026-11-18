#![no_main]
use libc_abi_tools::elf::Metadata;
use libfuzzer_sys::fuzz_target;
fuzz_target!(|bytes: &[u8]| {
    if let Ok(text) = std::str::from_utf8(bytes) {
        if let Ok(libraries) = Metadata::from_tsv(text) {
            for library in libraries {
                let _ = Metadata::from_tsv(&library.to_tsv());
            }
        }
    }
});
