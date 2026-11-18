// Compile the same private table decoder used by kernel-abi-tools, without
// adding a fuzz feature or parser API to any published crate.
#[path = "../../crates/kernel-abi-tools/src/data.rs"]
pub mod kernel_data;

// Private source modules are compiled directly so fuzzing uses production code
// without adding public APIs or fuzz dependencies to published packages.
pub use libc_abi_tools::{elf, stub, version};
#[allow(dead_code)]
#[path = "../../src/elf.rs"]
mod clock_elf;
#[allow(dead_code)]
#[path = "../../src/libgcc.rs"]
mod recorded_libgcc;

pub fn clock_copy(libc: &[u8], rt: &[u8]) -> Result<Option<Vec<u8>>, String> {
    clock_elf::libc_clock_copy(libc, rt)
}

pub fn recorded_interface(text: &str) {
    let _ = recorded_libgcc::parse(text);
}
