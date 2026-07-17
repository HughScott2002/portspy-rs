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
Hardcode the `<user>` half too for now (just type your username, or drop that
part entirely) — wiring the *real* logged-in user is an optional extension at the
end of the lesson. This lesson is only about the hostname.

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

- `char *name`: is that pointer `*const c_char` or `*mut c_char`? Ask yourself
  **who writes through it** — you, or C? (This is *the* decision of the lesson.)
- the **return type** `int`: C's `int` maps to `std::os::raw::c_int` (which is
  `i32` on this machine, but write `c_int` — it documents "this is C's int"). The
  existing declarations in `ffi.rs` return a pointer and a `u32`, so this one's
  new — you'll add `use std::os::raw::c_int;` (next to the existing
  `use std::os::raw::c_char;`).
- `size_t`: the Rust type for "an unsigned, pointer-sized length" is **`usize`**
  — the same one `portspy-wasm`'s `alloc(len: usize)` uses. Use that. (The *why*
  — that it's sized to the machine's pointer width — is the thing worth
  understanding; the answer is just `usize`.)

> ⚠️ **Trap — `*const` vs `*mut`.** `*const` = "only read through this." `*mut` =
> "may write through this." C *writes* the hostname into your buffer, so it must
> be `*mut c_char`. Important honesty: Rust **cannot check your `extern`
> declaration against C's real signature** — it trusts you. So declaring `*const`
> here wouldn't reliably produce a compiler error (a `*mut` can be coerced to
> `*const`); it would just be a *lie* that lets C write through a pointer you
> promised was read-only — exactly the kind of silent bug Lesson 6's tools exist
> to catch. Getting the pointer's mutability right is *your* job, from first
> principles, not the compiler's.

> 🧭 **Side quest (Rust): `c_char` vs `u8`.** C's `char` maps to
> `std::os::raw::c_char`, which is a *platform alias* — on x86-64 Linux it's
> **`i8`** (signed), because C leaves `char`'s signedness up to the platform. Your
> buffer is `[0u8; N]` (bytes are `u8`), so `u8` and `c_char` (`i8`) are different
> types to the compiler even though both are one byte. That's why you'll cast the
> buffer pointer somewhere — notice *where*, and know *why*.

### ✅ CHECKPOINT — does the declaration compile?
```sh
cargo build -p portspy-proc
```
A declaration produces no *output*, but it still has to be valid Rust. A green
build (an "unused function" warning is fine — you haven't called it yet) means
your `extern` line is **well-formed Rust**. Note what it does *not* prove:
compiling can't check your signature against C's real one — that ABI match (right
pointer mutability, right types) is the manual check you did against the man page.
Still, a first hand-written bridge to C that compiles is a real payoff.

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

> 🧭 **Side quest (Rust): handing C a buffer you own.** This mechanic is new —
> the existing `getpwuid` code never does it (libc handed *it* a pointer; here
> *you* provide one). The shape:
> ```rust
> let mut buffer = [0u8; HOSTNAME_MAX];   // N writable bytes on the stack
> let pointer = buffer.as_mut_ptr();      // *mut u8 = address of the first byte
> ```
> `buffer.as_mut_ptr()` is the scratchpad address you lend C. It's a `*mut u8`,
> but your `extern` declares the parameter as `*mut c_char`, so you'll cast:
> `pointer as *mut c_char`. (That's the `c_char` vs `u8` bump from Loop 3, now in
> practice.) The `unsafe` call then looks like
> `gethostname(buffer.as_mut_ptr() as *mut c_char, buffer.len())`.

Now wire it into the header. Two small steps, because of how Rust modules work:

1. Your `hostname()` lives in the **private** `ffi` module (`mod ffi;` in
   `crates/portspy-proc/src/lib.rs`), so marking the function `pub` isn't enough
   — the module hides it. Open that `lib.rs` and add a line right next to the
   existing `pub use ffi::is_root;`:
   ```rust
   pub use ffi::hostname;
   ```
   Now `portspy_proc::hostname()` exists.
2. In `run_print` (`main.rs`), replace the `"todo-hostname"` placeholder with a
   call to `portspy_proc::hostname()`.

(That module-visibility rule — `pub` function still hidden by a private module —
is a common Rust gotcha and comes back in Lesson 5.)

### ✅ CHECKPOINT
```sh
make print
```
Does a hostname appear? It might have a **tail of zero bytes after it** (they may
show as nothing or as `\0`) — that's expected, and it's Loop 5. If it *crashes* or
is totally empty, jump to Stuck?.

---

## Loop 5 — Turn the buffer back into a real `String`

C wrote the name and terminated it with a **NUL byte** (`\0`). Your buffer is 256
bytes but the name might fill only the first 12; the rest is the zero bytes you
initialized it with (`[0u8; 256]`). Either way, "how long is the real string?" is
a question you must answer — **stop at the first NUL** instead of treating all 256
bytes as the name.

You have two ways to do this — pick whichever you're comfortable with:

- **(a) The plain-Rust way (recommended for a first time — no extra `unsafe`).**
  Find the first NUL byte, then take everything before it:
  ```rust
  let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
  let name = String::from_utf8_lossy(&buffer[..end]).into_owned();
  ```
  `position(...)` gives you the index of the first `0`; `&buffer[..end]` is the
  real bytes; `from_utf8_lossy` turns them into a `String`.
- **(b) The C-string way (reuses what you saw).** `username_for_uid` in `ffi.rs`
  uses `CStr::from_ptr(...)` to read a NUL-terminated C string. You can do the
  same with `CStr::from_ptr(buffer.as_ptr() as *const c_char)` — but note that's
  *another* `unsafe` pointer cast, which is why (a) is the gentler first pass.

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

- Confirm the shape is clean: `pub fn hostname() -> String` with the `unsafe`
  *inside* it, so callers see no unsafe at all. (You already exposed it via
  `pub use ffi::hostname;` in Loop 4.)
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
