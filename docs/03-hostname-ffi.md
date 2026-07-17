# Lesson 3 — Calling C yourself: the hostname feature

← [Lesson 2](./02-testing.md) · [Index](./README.md) · Next: [Lesson 4 — Assertions](./04-assertions.md) →

**Goal:** hand-write your own call into the C library, from scratch. You'll add
the machine's **hostname** to portspy's output. This is the heart of the course —
the moment "calling a lib" stops being abstract.

**After this lesson you can:** declare a C function to Rust, pass it a memory
buffer, call it inside `unsafe`, and turn its result back into a Rust `String` —
and explain every step.

This lesson is built as **short loops** on purpose: you'll put a *fake* hostname
on screen first (so you see the shape working), then swap in the real C call.

> **The "don't tell me where everything goes" rule:** this lesson gives you the
> map, the questions, and the traps — but not the finished code. You'll find the
> spots yourself (you toured the crates in Lesson 1). When you're stuck, the
> existing `getpwuid` code in `ffi.rs` is your template for the *shape*.

---

## Loop 1 — Fake it: get a header on screen

Before any C, make the *visible thing* appear. In the CLI's print path
(`crates/portspy-cli/src/main.rs`, look at `run_print`), print a header line
above the table — with a **hardcoded** placeholder for now:

```
portspy · <hostname-goes-here> · <user>
```

Put a literal string like `"todo-hostname"` where the hostname will go.

### ✅ CHECKPOINT
```sh
make print
```
You see your header with `todo-hostname` in it. **That's loop 1 done** — the
plumbing is in place; now you'll replace the fake with the real value. (This is
the "fake it, then make it" rule from the index: see the shape first.)

---

## Loop 2 — Learn just enough C to read the manual

You've never read C. Three facts make the manual legible:

1. **`char *name`** — the `*` means "pointer to." `char` in C is just "one
   byte." So `char *name` = "the address where some bytes start." It is **not** a
   string object; C has no such thing.
2. **The caller owns the buffer.** Because C has no growable string, a function
   that "returns text" can't hand you one. Instead *you* make the empty space and
   pass in (a) where it is and (b) how big it is; C fills it. **You are lending C
   a scratchpad.** This is the whole shape of `gethostname`.
3. **The return value is a status, not the data.** Many C functions return an
   `int`: `0` = success, `-1` = failure. The actual data landed in *your buffer*.
   (Contrast `getpwuid`, which returned the data itself as a pointer.)

### ✅ CHECKPOINT — read the man page
```sh
man 2 gethostname
```
Find the SYNOPSIS. You're looking for a prototype shaped like:
```c
int gethostname(char *name, size_t len);
```
Read what it says about `len` and about the return value. **Those two details
drive your code.** (No `man`? It's online: <https://man7.org/linux/man-pages/man2/gethostname.2.html>.)

> 🧭 **Side quest (C): pointers, in 90 seconds, for a TS dev.**
> In JS, a variable holds a value or a reference you can't see the address of. In
> C, memory is one giant array of bytes, and a *pointer* is just an index into
> that array (an address). `char *name` holds an address; `*name` means "the byte
> living at that address." Passing a pointer to a function lets the function read
> **and write** your memory directly — which is exactly how `gethostname` fills
> your buffer. If this is shaky, watch one short video ("C pointers explained")
> then come back — it pays off for the rest of your career.

---

## Loop 3 — Declare the function to Rust

Tell Rust that `gethostname` exists out in libc — the same kind of `extern`
promise already sitting in `ffi.rs` for `getpwuid`. Add your declaration to that
same `unsafe extern "C"` block. Two translation puzzles:

- `char *name`: is that pointer `*const u8`/`*const c_char` or `*mut ...`? Ask
  yourself **who writes through it** — you, or C? (This is *the* decision of the
  lesson. Get it wrong and the compiler will tell you.)
- `size_t`: which Rust integer is "an unsigned, pointer-sized length"? You've
  already used it in this project for lengths — grep for it.

> ⚠️ **Trap — `*const` vs `*mut`.** `*const` = "I promise only to read through
> this." `*mut` = "I may write through this." C *writes* the hostname into your
> buffer. Lend it a `*const` and you've lied; the compiler (or worse, runtime)
> will object. This one's worth getting from first principles, not guessing.

> 🧭 **Side quest (Rust): `c_char` vs `u8`.** C's `char` maps to
> `std::os::raw::c_char`, which is *not the same type* as Rust's `u8` even though
> both are one byte. Why the ceremony? Because `char`'s signedness is
> platform-defined in C, so Rust keeps it a distinct type to stop you assuming.
> You'll likely need a cast somewhere — notice *where*, and know *why*.

There's nothing to run yet — declarations produce no output. On to the buffer.

---

## Loop 4 — Make the scratchpad and call it

Now the genuinely new-to-you part: you allocate a fixed lump of bytes and lend C
a pointer to it.

1. **Make N writable bytes** and get a *raw pointer* to the start. Figure out (a)
   what Rust type gives you N mutable bytes and (b) how to get a `*mut` pointer
   out of it. How big? Hostnames have a max length; a **named `const`** of 256 is
   plenty (TigerStyle: name the limit, don't scatter `256` around).
2. **Call `gethostname` inside `unsafe { }`.** Same reasoning as the existing
   code: *you* vouch that the signature is honest and the buffer is valid.
3. **Check the return value BEFORE trusting the buffer.** What does the man page
   say `-1` means? Handle it (return an empty string, say) rather than reading a
   buffer C never filled.

Wire the result into the header from Loop 1, replacing `"todo-hostname"`.

### ✅ CHECKPOINT
```sh
make print
```
Does a hostname appear? It might have **junk or zeros after it** — that's
expected, and it's Loop 5. If it *crashes* or is totally empty, jump to Stuck?.

---

## Loop 5 — Turn the buffer back into a real `String`

C left you bytes with a **NUL byte** (`\0`) marking the end. Your buffer is 256
bytes but the name might fill only the first 12; the rest is garbage. So "how
long is the string?" is a real question you must answer — stop at the NUL.

Good news: you've **already seen** the tool for "raw bytes ending in NUL → Rust
string." Look at how `username_for_uid` in `ffi.rs` turns libc's C string into a
Rust `String`. Reuse that idea.

### ✅ CHECKPOINT — the real thing, verified
```sh
make print         # your header now shows a clean hostname
hostname           # the system's own answer
```
They should **match**. If they do, your buffer round-trip is correct and you have
officially called C by hand. 🎉

> ⚠️ **Trap — forgetting the NUL.** If you convert the whole 256-byte buffer
> instead of stopping at the NUL, you get the hostname followed by a tail of
> zeros/junk. Now you know that smell for life.

---

## Loop 6 — Lock it in (TigerStyle + a test)

You built a safety net in Lesson 2; use it.

- Expose a small safe function (like `is_root` / `username_for_uid` already are)
  — the `unsafe` stays *inside* it, callers see a clean `fn hostname() -> String`.
- Add a `const` for the buffer size with a comment on *why* that size.
- Write a test: assert `hostname()` is **non-empty** on a normal machine. (You
  can't assert the exact value — it's machine-specific — but "non-empty and no
  panic" is a real guard, and it exercises the whole `unsafe` path.)

### ✅ CHECKPOINT
```sh
cargo test -p portspy-proc
```
Green. Your FFI code now has a test around it.

---

## 🎯 Done when
- [ ] `make print` shows a header whose hostname **matches** `hostname`.
- [ ] The `unsafe` is contained in one small safe function.
- [ ] The buffer size is a named `const`; the `-1` return is handled.
- [ ] There's a passing test that calls your `hostname()`.

## 🆘 Stuck?
- **Compiler error about `*const` vs `*mut`** — good, that's the lesson. C writes
  into the buffer, so the pointer must be `*mut`. Re-read Loop 3's trap.
- **`c_char`/`u8` mismatch** — you need a cast where the buffer pointer meets the
  `char *` parameter. Match the type in your `extern` declaration.
- **It crashes / segfaults** — you probably passed a bad pointer or the wrong
  length. Make sure the length you pass C is your buffer's real size, and that
  the buffer outlives the call. (Lesson 6 gives you Valgrind to catch exactly
  this — but first get it not-crashing by re-checking pointer + length.)
- **Hostname has junk after it** — that's the NUL issue, Loop 5. Stop at the NUL.
- **Totally stuck on the shape** — open `ffi.rs` and read `username_for_uid` line
  by line; your `hostname()` is the same skeleton with a buffer you own.

## Commit it
```sh
git add -A && git commit -m "lesson 3: hostname via hand-written libc FFI (gethostname)"
```

Next: [Lesson 4 — Making it bulletproof: assertions](./04-assertions.md)
