# Lesson 2 — Your safety net: testing

← [Lesson 1](./01-orientation.md) · [Index](./README.md) · Next: [Lesson 3 — Hostname FFI](./03-hostname-ffi.md) →

**Goal:** get a **green safety net** under the code _before_ you start changing
scary systems code in Lesson 3. Tests are the fastest feedback loop you have —
change something, run one command, instantly know if you broke it.

**After this lesson you can:** write and run Rust tests, and you'll have real
tests covering portspy's pure logic.

Estimated loops: 5 (one per test — each ends in a green `ok`).

> Why testing _before_ the FFI feature? Because in Lesson 3 you'll write `unsafe`
> code that can fail in confusing ways. A safety net means when something breaks,
> `cargo test` points at _which_ thing, instantly. TigerStyle says: test
> everything — start now, while the code is simple.

---

## 1. How tests work in Rust (vs. jest)

Coming from TS, think `jest` — but **built into the toolchain**. No install, no
config. A test is a function marked `#[test]` that **passes unless it panics**.
`assert_eq!(a, b)` panics if `a != b`. You run them all with `cargo test`.

Tests usually live in the _same file_ as the code they test, inside a special
block:

```rust
#[cfg(test)]          // "only compile this when running tests"
mod tests {
    use super::*;     // pull in everything from this file, including private fns

    #[test]
    fn two_plus_two() {
        assert_eq!(2 + 2, 4);
    }
}
```

`#[cfg(test)]` means this block adds **nothing** to your real binary — it only
exists during `cargo test`. That's why tests can live right next to the code.

### ✅ CHECKPOINT — see the loop work

Add exactly that `mod tests` block to the bottom of
`crates/portspy-model/src/wire.rs`, then:

```sh
cargo test -p portspy-model
```

You should see `test tests::two_plus_two ... ok` and `1 passed`. **Green. That's
the loop.** Now delete `two_plus_two` and let's test real things.

---

## 2. Test the pure functions (your exercises)

The best first tests are on **pure functions** — input in, value out, no OS
involved. portspy has several. Write these yourself; each is one quick loop.

Keep your tests in the `#[cfg(test)] mod tests` block at the bottom of the
relevant file.

### Exercise A — `wire::decode` ignores malformed lines

In `wire.rs`. The format is 8 tab-separated columns per line. Write a test that:

- `decode("")` returns 0 records.
- a line with the wrong number of tabs is skipped (returns 0 records).
- one well-formed line (8 columns, tabs between) decodes to 1 record with the
  port you put in it.

> 🧭 **Side quest (Rust): how do I build a tab/newline string in a test?**
> Use escape codes in a normal string: `"tcp\t127.0.0.1\t3000\tLISTEN\t7\tnode\tjr\t"`
> — `\t` is a tab, `\n` a newline. Count your tabs: 8 fields need 7 tabs.

### Exercise B — `decode_ipv4` reverses bytes correctly

In `net_tcp.rs`. `/proc` writes IPv4 addresses as little-endian hex, so
`"0100007F"` must come out as `"127.0.0.1"`. Write a test asserting exactly that.
This is the trickiest parser in the project — a test here is worth a lot.

> ⚠️ **Visibility trap.** `decode_ipv4` and `parse_listening` are _private_. A
> `#[cfg(test)] mod tests` block _inside the same file_ can still see them via
> `use super::*` — so put your test there. (If you ever test from another file,
> you'd need to widen visibility to `pub(crate)`; prefer the smallest visibility
> that works.)

### Exercise C — `view::sort` and `view::filter`

In `view.rs`. Build a small `Vec<Record>` by hand and assert:

- after `sort(&mut v, SortKey::Port)`, the ports are ascending.
- `filter(&v, "")` keeps all; `filter(&v, "node")` keeps only matching rows;
  filtering is case-insensitive (`"NODE"` matches too).

> 🧭 **Side quest (Rust): building a `Record` in a test.**
> `Record` has public fields, so you can construct one directly:
> `Record { proto: "tcp".into(), addr: "127.0.0.1".into(), port: 3000, state: "LISTEN".into(), pid: 7, process: "node".into(), user: "jr".into(), cmd: "".into() }`.
> `.into()` turns a `&str` into a `String`. Tedious? Write a tiny helper fn
> _inside_ your test module that takes a port + name and fills the rest with
> defaults. That's normal and good.

### ✅ CHECKPOINT — run the whole net

```sh
cargo test
```

(no `-p` — runs every crate). You want a line like `test result: ok. N passed`.
Every green line is a fact about your code that stays true forever.

---

## 3. Bonus: a test that catches a _rule_, not a case

Most tests check one example. A stronger kind checks a rule that must hold for
_any_ input. The natural one here: **decoding what you encoded gives back what
you started with.**

Try writing `encode_then_decode_roundtrips`: build a couple of records, `encode`
them, `decode` the result, assert you get the same count and fields back.

You'll hit two real snags (this is the point):

- `Record` isn't comparable with `assert_eq!` yet — it needs
  `#[derive(PartialEq)]`. Adding that to the struct in `record.rs` is a fair
  call; do it if it helps.
- Records whose fields contain tabs/newlines **won't** round-trip exactly,
  because `encode` sanitizes those. That's the format's contract, not a bug —
  build your test records without control characters.

This "rule for any input" idea is exactly what the fuzzer will automate in
Lesson 5. You're planting the seed now.

---

## 🎯 Done when

- [ ] `cargo test` runs and every test passes.
- [ ] You have at least: one `wire::decode` test, the `decode_ipv4` test, and one
      `view` test.
- [ ] You understand why the tests live _inside_ the file they test.

## 🆘 Stuck?

- `cannot find function decode` → add `use super::*;` at the top of your
  `mod tests`.
- Test won't compile because of a private field/function → it's likely in a
  _different_ file than your test; move the test into the file that defines it.
- `assert_eq!` won't compile on a `Record` → add `PartialEq` to the derive line
  it already has (`#[derive(Clone, Debug)]` → `#[derive(Clone, Debug, PartialEq)]`).
  It already derives `Debug`, so don't add a second one — that's a compile error.

## Commit it

```sh
git add -A && git commit -m "lesson 2: tests for wire, net_tcp, and view logic"
```

Next: [Lesson 3 — Calling C yourself: the hostname feature](./03-hostname-ffi.md)
