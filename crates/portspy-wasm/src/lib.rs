//! portspy-wasm
//!
//! The browser cannot read /proc, so the native agent does that and sends the
//! port list as text. What the browser DOES do -- sorting and filtering as you
//! type -- runs here, in WebAssembly, using the very same portspy-model code
//! the native binary links against.
//!
//! ## The boundary problem
//!
//! WebAssembly and JavaScript can only pass NUMBERS to each other. They cannot
//! pass a Rust String or a JS string directly. So the deal is:
//!
//!   * JS asks us for a scratch buffer:      ptr = alloc(len)
//!   * JS writes its text into wasm memory at ptr
//!   * JS calls process(...) with those numbers; we do the real work
//!   * we return where our answer lives, packed as one 64-bit number
//!   * JS reads the answer out of wasm memory, then frees both buffers
//!
//! Every function here is the thin translation layer around that dance. The
//! actual logic is all `portspy_model`.

use core::slice;
use core::str;

use portspy_model::SortKey;
use portspy_model::view;
use portspy_model::wire;

/// Give JavaScript a block of `len` bytes inside wasm memory and return where
/// it starts. JS will fill it with text before calling `process`.
///
/// We allocate a boxed slice (exact length) so `dealloc` can free it precisely.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    // A zeroed buffer of exactly `len` bytes.
    let buffer = vec![0u8; len].into_boxed_slice();

    // Hand ownership to the raw world. JS must give it back via `dealloc`.
    let raw: *mut [u8] = Box::into_raw(buffer);
    raw as *mut u8
}

/// Free a block previously handed out by `alloc` (or returned by `process`).
///
/// `len` must be the same length that block was created with.
#[unsafe(no_mangle)]
pub extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }

    // SAFETY: JS promises ptr/len came from one of our allocations and is freed
    // exactly once. Rebuilding the Box lets Rust drop it and reclaim the memory.
    unsafe {
        let slice = slice::from_raw_parts_mut(ptr, len);
        let boxed: Box<[u8]> = Box::from_raw(slice);
        drop(boxed);
    }
}

/// Sort and filter the port list. This is the whole reason wasm is here.
///
/// Inputs (all numbers, because that is all the boundary allows):
///   in_ptr, in_len     - the wire-encoded port list JS wrote into our memory
///   sort_key           - 0 port, 1 pid, 2 process, 3 user (see SortKey)
///   filter_ptr, f_len  - the search text JS wrote into our memory ("" = all)
///
/// Output: one 64-bit number that packs WHERE the answer is and HOW LONG it is:
///   high 32 bits = length, low 32 bits = pointer. JS unpacks both, reads the
///   bytes, and frees them with `dealloc`.
#[unsafe(no_mangle)]
pub extern "C" fn process(
    in_ptr: *const u8,
    in_len: usize,
    sort_key: u32,
    filter_ptr: *const u8,
    filter_len: usize,
) -> u64 {
    // Turn the two raw (ptr, len) inputs into Rust string slices we can use.
    let input_text = read_utf8(in_ptr, in_len);
    let needle = read_utf8(filter_ptr, filter_len);

    // ---- the real work, all in portspy-model ----
    let all_records = wire::decode(input_text); // text  -> records
    let mut shown = view::filter(&all_records, needle); // keep matches
    view::sort(&mut shown, SortKey::from_u32(sort_key)); // reorder
    let output_text = wire::encode(&shown); // records -> text
    // ----------------------------------------------

    // Copy the answer into a fresh buffer JS will later free, and report where.
    write_output(output_text)
}

/// View a raw (ptr, len) pair from wasm memory as a &str.
///
/// If the bytes are not valid UTF-8 (they always are here, but we check anyway)
/// we return "" rather than risk a panic across the boundary.
fn read_utf8<'a>(ptr: *const u8, len: usize) -> &'a str {
    if ptr.is_null() || len == 0 {
        return "";
    }

    // SAFETY: JS guarantees ptr..ptr+len is a buffer it allocated via `alloc`
    // and has not freed yet, so the region is valid for reads.
    let bytes = unsafe { slice::from_raw_parts(ptr, len) };

    match str::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => "",
    }
}

/// Move `text` into a buffer JS owns, and pack its (pointer, length) into a u64.
fn write_output(text: String) -> u64 {
    let bytes = text.into_bytes().into_boxed_slice();
    let len = bytes.len();

    let raw: *mut [u8] = Box::into_raw(bytes);
    let ptr = raw as *mut u8;

    // Pointers are 32-bit in wasm32, so a pointer and a length both fit in the
    // two halves of a 64-bit number. High half = length, low half = pointer.
    let ptr_bits = ptr as u64;
    let len_bits = len as u64;
    (len_bits << 32) | ptr_bits
}
