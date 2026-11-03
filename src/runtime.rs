//! Experimental runtime archive preparation, separate from application linking.
//!
//! Preparing an archive proves neither mixed-runtime interoperability nor a safe
//! executable default. No provider descriptor, raw evidence or linker policy changes.
mod archive;
mod inputs;
pub mod llvm_unwind;
mod sources;
