# Lesson 6 — Did I corrupt anything? Valgrind & Miri

← [Lesson 5](./05-fuzzing.md) · [Index](./README.md) · Next: [Lesson 7 — Going deeper](./07-going-deeper.md) →

**Goal:** prove your `unsafe` code — especially the Lesson 3 FFI — didn't quietly
scribble on memory it shouldn't. Fuzzing catches *crashes*; these tools catch
*silent corruption*, the scariest class of bug.

**After this lesson you can:** run Valgrind and Miri, know which to use where, and
you'll have `make valgrind` / `make miri` as one-command checks.

Estimated loops: 2.

---

## 1. Why crashes aren't enough

Recall from the unsafe/UB discussion: broken `unsafe` code is **not guaranteed to
crash**. It can write one byte past a buffer, "work" fine, and corrupt an
unrelated variable that blows up much later — or never, until a different
machine. Your tests and fuzzer might all pass while memory is subtly wrong. You
need tools whose *entire job* is watching every memory access as it happens.

Two of them, with different strengths:

| Tool | What it is | Use it on |
|------|------------|-----------|
| **Valgrind** | a referee watching the *real* program run; works through FFI into C | your libc `unsafe` (`portspy-proc`) |
| **Miri** | a robot that *simulates* your Rust against a strict rulebook; can't call real C | your pure-Rust `unsafe` (`portspy-wasm`) |

> 🧭 **Side quest (concept): why can't Miri check the FFI?**
> Miri doesn't run real machine code — it *interprets* your Rust in a careful
> imaginary machine. A call to libc's `getpwuid` is real outside code it can't
> execute, so Miri stops with "unsupported: can't call foreign function." That's
> not a setup problem — it's the boundary of what a simulator can do. So: Miri for
> pure Rust, Valgrind for anything touching C.

---

## 2. Loop 1 — Valgrind on the real binary (catches FFI bugs)

Valgrind runs your normal (debug) binary and blows the whistle on invalid
reads/writes, use-after-free, and leaks — the exact failure mode if your Lesson 3
buffer length or pointer was wrong.

### ✅ CHECKPOINT — install and run
```sh
sudo apt install valgrind        # if you don't have it
cargo build -p portspy-cli       # debug build (better diagnostics than release)
valgrind ./target/debug/portspy print
```
Read the summary at the bottom. You want:
```
ERROR SUMMARY: 0 errors from 0 contexts ...
```
**Zero errors = your FFI didn't touch memory it shouldn't.** If instead you see
`Invalid write of size 1` pointing near your `gethostname` code — congratulations,
you found a real Category-B bug the tests couldn't. Fix the buffer size/length and
re-run until it's clean.

> Note: Valgrind may report a few "still reachable" bytes at exit from libc/Rust
> runtime — those are harmless. Focus on *Invalid read/write* and *definitely
> lost*.

---

## 3. Loop 2 — Miri on the pure-Rust unsafe (the wasm crate)

`portspy-wasm` has `unsafe` with no C: the raw-pointer handling lives in
`dealloc` and in the `read_utf8` / `write_output` helpers. That's Miri's home
turf. Miri needs a *test* to have something to run, so add a tiny one that drives
the allocator round-trip:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_then_dealloc_roundtrips() {
        let len = 8;
        let ptr = alloc(len);
        // write into the buffer we were handed, then give it back
        unsafe {
            let bytes = std::slice::from_raw_parts_mut(ptr, len);
            bytes.fill(0);
        }
        dealloc(ptr, len);   // Miri verifies this frees exactly what alloc made
    }
}
```

> ⚠️ **Do NOT test `process` under Miri.** `process` packs its result as
> `(len << 32) | ptr`, which assumes **32-bit wasm pointers**. Miri runs on your
> **64-bit** host, where a real heap pointer needs all 64 bits — packing it into
> 32 truncates it, and unpacking then frees a bogus address (real UB). That's a
> quirk of the host, not a bug in your code, so keep the Miri test to
> `alloc`/`dealloc`, whose `ptr`/`len` are passed as honest separate values.

### ✅ CHECKPOINT
```sh
rustup +nightly component add miri     # one-time
cargo +nightly miri test -p portspy-wasm
```
A clean pass means Miri found no undefined behavior in the allocator's pointer
handling. If it prints something like "out-of-bounds pointer arithmetic" or
"using uninitialized memory," it's pointing straight at a real UB bug — with a
stack trace. Fix and re-run.

> 🆘 If the pinned nightly lacks the `miri` component, install a fresh one:
> `rustup toolchain install nightly --component miri`, then use
> `cargo +nightly miri ...`.

---

## 4. Make it one command each (build the targets)

Back in Lesson 1 you saw the `Makefile` is just readable shortcuts. Add two so
these checks are frictionless forever. Open the `Makefile` and add targets in the
same style as the existing ones:

- `make valgrind` → build the debug binary, then run
  `valgrind ./target/debug/portspy print`.
- `make miri` → run `cargo +nightly miri test -p portspy-wasm`.

Add them to the `.PHONY` line and to the `help` text so future-you remembers.

### ✅ CHECKPOINT
```sh
make valgrind
make miri
```
Both run with a single word. That's the loop closed: whenever you touch `unsafe`,
one command tells you if you broke memory.

> 🧭 **Side quest (optional): AddressSanitizer.** There's a third tool,
> **ASan**, that recompiles your code with memory checks baked in and catches
> overruns through FFI *with a stack trace* (Valgrind is slower and less precise;
> ASan needs a rebuild). Try it if curious:
> `RUSTFLAGS="-Zsanitizer=address" cargo +nightly run -p portspy-cli -- print`.
> Not required — Valgrind already covers you here.

---

## 🎯 Done when
- [ ] `valgrind ./target/debug/portspy print` reports **0 errors**.
- [ ] `cargo +nightly miri test -p portspy-wasm` passes.
- [ ] `make valgrind` and `make miri` exist and work.
- [ ] You can say, in one sentence each, why Valgrind fits the FFI and Miri fits
      the wasm crate.

## 🆘 Stuck?
- Valgrind flags errors *inside libc, not your code* → look at the call stack; if
  the top frame is your `hostname()`/`ffi.rs`, it's yours. Deep-in-libc-only noise
  at exit is usually harmless (see the note above).
- Miri errors on a libc call → you pointed it at the wrong crate. Keep it on
  `portspy-wasm`; Miri can't cross into C (see the side quest).
- `miri` component won't add → install a nightly that has it (Stuck box in §3).

## Commit it
```sh
git add -A && git commit -m "lesson 6: valgrind + miri clean; add make valgrind / make miri"
```

Next: [Lesson 7 — Going deeper: syscalls, and where next](./07-going-deeper.md)
