# Learning: testing and fuzzing portspy

> A guided path, not a spec. You'll write plain tests first, turn one into a
> property test, then set a fuzzer loose on the parsers. Everything here targets
> code that already exists, so you can see real results (and maybe real bugs).

## Part 0 — the mental model

- A **test** pins down behaviour: "given input X, I expect output Y."
- A **property test** pins down a *rule that must always hold*: "for **any**
  input, this is true." (e.g. `decode(encode(x)) == x`.)
- A **fuzzer** is a robot QA tester that throws millions of random and nasty
  inputs at one function, watching for anything that makes it crash, hang, or
  break a property. It's great at finding the input you'd never think to try.

The workflow they form together:
**fuzz finds a bad input → save it → paste it into a unit test → fix the bug →
the test guards it forever.**

## Part 1 — your first tests (`cargo test`)

In Rust, tests are built into the toolchain (like `jest`, but no install). A test
is a function marked `#[test]` that panics if something's wrong. Unit tests live
in the same file as the code, inside a block that only compiles during testing:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_decodes_to_no_records() {
        let records = decode("");
        assert_eq!(records.len(), 0);
    }
}
```

`#[cfg(test)]` = "only compile this when testing," so it adds nothing to the real
binary. `use super::*;` pulls in the module's functions (including private ones).

Run everything with:
```
cargo test
```

### Your exercises (pure functions — the easy, high-value wins)

Write a handful of these yourself. All the targets are I/O-free, so they're
trivial to check:

1. **`portspy-model` wire** (`crates/portspy-model/src/wire.rs`)
   - a line with the wrong number of columns (not 8) is skipped by `decode`.
   - a field containing a tab or newline gets sanitized by `encode` (the
     separators must survive intact).
2. **`portspy-model` view** (`crates/portspy-model/src/view.rs`)
   - `sort` by `Port` actually orders ascending.
   - `filter` with an empty needle keeps everything; a specific needle keeps
     only matches; matching is case-insensitive.
3. **`portspy-proc` parser** (`crates/portspy-proc/src/net_tcp.rs`)
   - feed one real `/proc/net/tcp` line (copy one from your machine) and assert
     the port, address, and inode come out right.
   - `decode_ipv4("0100007F")` == `"127.0.0.1"` (this is where the little-endian
     byte-reversal gets verified).

> Note: `net_tcp`'s functions are currently private. To test them you'll either
> add tests *inside* that file (so `use super::*` can see them) or make them
> `pub(crate)`. Deciding which is part of the exercise — prefer the smallest
> visibility that lets the test reach them.

## Part 2 — a property test (the bridge)

Some rules hold for *every* input, not one example. The natural one here:

> decoding what you encoded gives back what you started with.

Sketch:
```rust
#[test]
fn encode_then_decode_roundtrips() {
    let original = vec![/* a couple of hand-built Records */];
    let text = encode(&original);
    let back = decode(&text);
    assert_eq!(back.len(), original.len());
    // then compare fields (Record isn't PartialEq yet — deriving it would help)
}
```

Two things you'll discover writing this:
- `Record` doesn't implement `PartialEq`, so `assert_eq!` on whole records won't
  compile yet. Deciding whether to `#[derive(PartialEq)]` on it is a real design
  call — do it if it helps tests and costs nothing.
- Fields that contained a tab/newline **won't** round-trip exactly, because
  `encode` sanitizes them. That's not a bug — it's the format's contract. A good
  property test respects it (build your test records without control characters,
  or assert the *sanitized* form comes back).

This is the exact question a fuzzer will ask automatically next.

## Part 3 — fuzzing the parsers (`cargo-fuzz`)

A fuzzer eats **untrusted input**, and you have several parsers that do exactly
that. Best targets, in order of payoff:

- `wire::decode` — takes arbitrary text.
- `net_tcp::parse_listening` — parses whatever's in `/proc` (or, fuzzing,
  whatever garbage the robot invents).
- `decode_ipv4` / `decode_ipv6` — take hex strings of an *assumed* length.
- `request_path` in `portspy-http` — parses bytes straight off the network
  (real attacker surface).

### Setup (one time)
```
cargo install cargo-fuzz          # the tool
rustup toolchain install nightly  # libFuzzer needs nightly
cargo fuzz init                    # scaffolds a fuzz/ crate
cargo fuzz add decode_wire         # create one fuzz target
```

### What a fuzz target looks like

A fuzz target is a function handed a slice of random bytes; it feeds them into
the thing under test. The goal is **"never crash, no matter the input"** — you
usually don't check the output at all:

```rust
// fuzz/fuzz_targets/decode_wire.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // interpret the random bytes as text and try to decode them
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = portspy_model::wire::decode(text);
        // it just must not panic
    }
});
```

Run it — it loops forever, getting smarter about which inputs reach new code
(coverage-guided):
```
cargo fuzz run decode_wire
```

### Fuzzing a *property*, not just crashes

You can put the round-trip assertion *inside* the target. Now the robot hunts
for any input that breaks your invariant:
```rust
fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let records = portspy_model::wire::decode(text);
        let reencoded = portspy_model::wire::encode(&records);
        let again = portspy_model::wire::decode(&reencoded);
        assert_eq!(records.len(), again.len()); // must be stable
    }
});
```

### Sharp edges to point the fuzzer at (see if it finds them)

These are real spots in the current code worth aiming at — treat them as "does
the fuzzer catch this?" challenges:

- **`decode_ipv4` fixed-length indexing** (`net_tcp.rs`): it checks
  `len() == 8`, then slices `[start..start+2]`. Is every path guarded? Point a
  fuzz target at `parse_listening` with random lines and see if any slice
  panics. (If it's actually safe, the fuzzer finding *nothing* is a real result
  too — that's a proof, cheaply earned.)
- **The 8-column assumption** in `wire::decode`: a line with 7 or 9 tabs. Should
  be skipped cleanly — confirm the fuzzer agrees.
- **`request_path`** on a request with no spaces, or a lone `\r`, or empty
  input.

### When it finds something

The fuzzer **saves the crashing input to a file** (under `fuzz/artifacts/`) and
prints the panic. Replay it deterministically with:
```
cargo fuzz run decode_wire fuzz/artifacts/decode_wire/crash-abc123
```
Then close the loop: copy those exact bytes into a `#[test]` in the parser's
file, fix the bug, and the test guards it forever.

## How this connects to the other docs

- Running a fuzz target under **Miri** (pure-Rust unsafe) or the binary under
  **Valgrind** (FFI unsafe) turns "did it crash?" into "did it corrupt memory?"
  — see [future-work.md](./future-work.md).
- **Assertions** (see [tigerstyle-assertions.md](./tigerstyle-assertions.md))
  make bugs surface earlier and louder, so the fuzzer trips them faster. A fuzzer
  plus liberal `assert!`s is a genuinely strong combination.

## Suggested order

1. Write 3–4 unit tests on the pure functions. Run `cargo test`. Get the green.
2. Write the round-trip property test; hit the `PartialEq` / sanitize wrinkles.
3. `cargo fuzz init`, add a `decode_wire` target, let it run a few minutes.
4. Point a target at `parse_listening`; aim at the sharp edges above.
5. Anything it finds → becomes a permanent unit test.
