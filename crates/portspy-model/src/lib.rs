//! portspy-model
//!
//! The shared vocabulary of the whole project. Three small modules:
//!
//!   record  - the `Record` type: one listening port and who owns it.
//!   wire    - turn records into text and back (the format sent to the browser).
//!   view    - sort and filter records (runs natively AND in wasm).
//!
//! Zero dependencies. Zero I/O. This is deliberately the only crate that both
//! the native binary and the WebAssembly module depend on.

pub mod record;
pub mod view;
pub mod wire;

// Re-export the common types at the crate root so callers can write
// `portspy_model::Record` instead of `portspy_model::record::Record`.
pub use record::Record;
pub use view::SortKey;
