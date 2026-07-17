# Lesson 5 — Breaking it on purpose: fuzzing

← [Lesson 4](./04-assertions.md) · [Index](./README.md) · Next: [Lesson 6 — Memory checkers](./06-memory-checkers.md) →

**Goal:** unleash a **robot that throws millions of random, nasty inputs** at your
parsers, watching for anything that makes them crash or break a rule. Then turn
any crash it finds into a permanent test.

**After this lesson you can:** write a fuzz target, run a coverage-guided fuzzer,
and fold its findings back into your test suite.

Estimated loops: 3 — and one of them is *watching a live counter go up*, which is
exactly the "stuff happening" you want.

---

## 1. What fuzzing is (one paragraph)

A fuzzer is an automated QA tester that feeds a function random and deliberately
horrible inputs — empty, gigantic, invalid UTF-8, one column short — as fast as
it can, and flags any input that **panics, hangs, or violates a property you
assert**. It's *coverage-guided*: it watches which branches of your code each
input reaches and mutates toward inputs that explore new paths. It finds the case
you'd never think to type.

**Why portspy is a perfect target:** fuzzing shines on code that eats *untrusted*
input, and you have several such parsers: `wire::decode`, `net_tcp::parse_listening`,
`decode_ipv4`, and the HTTP `request_path`.

> 🧭 **Side quest (concept): "coverage-guided" in plain terms.**
> Dumb fuzzing = pure random noise (rarely gets past the first `if`). Coverage-
> guided fuzzing keeps inputs that reached *new code* and mutates them, so it
> "learns" the shape of your parser and drills deep. That's why it finds real
> bugs in minutes, not millennia.

---

## 2. Set up `cargo-fuzz`

`cargo-fuzz` drives LLVM's libFuzzer. It needs the **nightly** toolchain.

### ✅ CHECKPOINT — install and scaffold
```sh
cargo install cargo-fuzz          # one-time
rustup toolchain install nightly  # libFuzzer needs nightly
cargo fuzz init                    # creates a fuzz/ crate in the workspace
```
You should now have a `fuzz/` directory. `cargo fuzz list` prints the (empty or
sample) target list without error.

> 🆘 If `cargo install cargo-fuzz` fails to build, you may need LLVM/clang
> installed (`sudo apt install clang`). libFuzzer ships with the Rust nightly
> sanitizer support, so the nightly toolchain is the key part.

---

## 3. Loop 1 — fuzz `wire::decode` (watch it run)

A **fuzz target** is a function handed a slice of random bytes; you feed them into
the code under test. The goal is simply: **never crash, no matter the input.**

Create a target:
```sh
cargo fuzz add decode_wire
```
Edit `fuzz/fuzz_targets/decode_wire.rs` to something like:
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // interpret the random bytes as text, then try to decode them
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = portspy_model::wire::decode(text);
        // no assertion needed: it just must not panic
    }
});
```
(You'll add `portspy-model` to `fuzz/Cargo.toml`'s dependencies — the scaffold
shows where.)

### ✅ CHECKPOINT — the counter
```sh
cargo fuzz run decode_wire
```
You'll see a live readout: `#12345 ... exec/s: 40000 ... cov: 210`. **That number
climbing is thousands of inputs per second hitting your parser.** Let it run ~60
seconds. No crash on `wire::decode`? Great — that's a *result*: your decoder
survives arbitrary text. `Ctrl-C` to stop.

---

## 4. Loop 2 — fuzz a *property*, not just crashes

You can bake a rule into the target so the robot hunts for inputs that break it —
this is your Lesson 2 round-trip idea, automated:
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
Run it the same way. If the fuzzer finds an input where the count changes, it's
found a real inconsistency in your format — exactly what you want to know.

---

## 5. Loop 3 — aim at the sharp edges

Add a target for `parse_listening` (feed it random text as fake `/proc` lines).
These spots are worth aiming at — treat them as "does the fuzzer catch this?"
challenges:

- **`decode_ipv4`'s fixed-length indexing** (`net_tcp.rs`): it checks
  `len() == 8`, then slices `[start..start+2]`. Is every code path that indexes
  actually guarded by a length check? Point the fuzzer at `parse_listening` and
  see if any slice panics.
- **The 8-column assumption** in `wire::decode`: lines with 7 or 9 tabs should be
  skipped cleanly.
- **`request_path`** (in `portspy-http`): a request with no spaces, a lone `\r`,
  or empty input.

> If the fuzzer finds **nothing** after a good run, that's a genuine result too —
> a cheap proof that path is robust. "No bug found" is information, not failure.

### When it *does* find something
libFuzzer saves the crashing input to a file (under `fuzz/artifacts/...`) and
prints the panic. Replay it deterministically:
```sh
cargo fuzz run decode_wire fuzz/artifacts/decode_wire/crash-<hash>
```
Then **close the loop**: copy those exact bytes into a `#[test]` in the parser's
file (Lesson 2 style), fix the bug, and that test guards it forever. Fuzzer finds
it once; the test stops it coming back.

---

## 🎯 Done when
- [ ] You ran a fuzz target and watched the exec/s counter climb.
- [ ] You have at least two targets (`decode_wire` + `parse_listening`).
- [ ] Either you found and fixed a crash (and wrote a regression test), **or** you
      have a clean run and understand that's a real result.

## 🆘 Stuck?
- `cargo fuzz` "requires nightly" → it invokes nightly itself; make sure
  `rustup toolchain install nightly` succeeded.
- Target can't find `portspy_model` → add it under `[dependencies]` in
  `fuzz/Cargo.toml` with a `path = "../crates/portspy-model"`.
- Runs but 0 exec/s / hangs immediately → your target is probably doing I/O or an
  infinite loop; keep the target body tiny and pure (parse only).

## Commit it
```sh
git add -A && git commit -m "lesson 5: cargo-fuzz targets for the parsers (+ regression tests)"
```

Next: [Lesson 6 — Did I corrupt anything? Valgrind & Miri](./06-memory-checkers.md)
