# Feature: show the machine's hostname (a guided FFI exercise)

> This is a **learning guide**, not a spec. It deliberately gives you the map,
> the questions, and the traps — not the answers. Work it top to bottom, look
> around the codebase to find your own footing, and let the compiler teach you.
>
> Branch: `feat/hostname-ffi`

## The goal

Add the machine's hostname to portspy — for example a header line on
`portspy print` like:

```
portspy · <hostname> · <user>
```

Small, visible, and it makes you *keep* the FFI muscle you're building.

## Why this feature (and not something easier)

The `getpwuid` call already in the project teaches **one** direction of FFI:
*C hands you a pointer, you read from it.*

`gethostname` teaches the **opposite and more common** direction:
*you hand C a pointer to your own memory, and C writes into it.*

That "lend C a scratchpad" pattern is everywhere in C. Different enough to
stretch you, small enough to finish in one sitting.

## The C you need first (you've never read C — start here)

1. **`char *name`** — the `*` means "pointer to". `char` in C is just "one
   byte". So `char *name` = "the address where some bytes start". It is **not**
   a string object; C has no such thing.

2. **The caller-owns-the-buffer idiom.** Because C has no growable string, a
   function that "returns text" can't hand you a `String`. Instead *you* create
   the empty space and pass in (a) where it is and (b) how big it is. C fills
   it. **You are lending C a scratchpad.** Sit with this — it's the unlock.

3. **The return value is a status, not the data.** Many C functions return an
   `int`: `0` = success, `-1` = failure. The real data landed in *your buffer*,
   not in the return. (Contrast `getpwuid`, which returned the data as a
   pointer.)

Now go read the man page:

```
man 2 gethostname
```

Read the SYNOPSIS. You'll find a prototype shaped like
`int gethostname(char *name, size_t len)`. Pay attention to what it says about
`len` and about the return value — those two details drive your code.

## The quest (questions, not answers)

Work these roughly in order. When stuck, look at the existing `getpwuid` block
as a template for the **shape**, not something to copy.

### 1. Where does this code belong?
Don't look it up — reason it out. The project already has exactly one file
whose job is "the only place that talks to C." Why was that a deliberate design
choice? Put your new declaration where that same reasoning points. Start your
search in whichever crate reads the OS.

### 2. Declare the function
Tell Rust that `gethostname` exists out in libc — the same kind of `extern`
promise you already have for `getpwuid`. Translate the C prototype into that
block. Two translation puzzles:

- `char *name`: is that pointer `*const` or `*mut`? Ask *who writes through
  it* — you, or C? (This is the crux. Get it wrong and the compiler will help.)
- `size_t`: which Rust integer is "an unsigned, pointer-sized length"? You've
  already used it elsewhere in this project for lengths.

### 3. Make the scratchpad
You need a chunk of writable bytes to lend to C, plus a raw pointer to its
start. As a TS dev this is the alien part: you allocate a fixed lump of memory
yourself. Figure out (a) what Rust type gives you N mutable bytes, and (b) how
to get a *raw pointer* out of it to pass across the boundary. How big? Hostnames
have a max length; a named `const` of 256 is plenty. (TigerStyle: name the
size.)

### 4. Call it — inside `unsafe` — and check the result
Same `unsafe { }` reasoning as before: *you* vouch that the signature is honest
and the buffer is valid. After the call, look at the **return value before you
trust the buffer**. What does the man page say `-1` means you should do?

### 5. Turn the buffer back into a Rust `String`
C left you bytes with a **NUL byte** (`\0`) marking the end — the buffer may be
256 bytes but the name only fills the first 12. So "how long is the string?" is
a real question you must answer. You've **already seen** the tool for "raw bytes
ending in NUL → Rust string" in the `getpwuid` code. Find it there and reuse the
idea.

### 6. Surface it
Expose a small safe function (like `is_root` / `username_for_uid` already are)
and call it from wherever the terminal header is printed. You choose the
wording.

## How you'll know it worked

```
make print
```

Compare against the real thing:

```
hostname
# or
cat /proc/sys/kernel/hostname
```

If they match, your buffer round-trip is correct.

## Traps left in on purpose (this is the good part)

- **`*const` vs `*mut`** on the buffer — lend C a read-only pointer to write
  through and the compiler complains. Read the error; it's teaching you.
- **Forgetting the NUL** — convert the *whole* 256-byte buffer instead of
  stopping at the NUL and you get a hostname with a tail of zeros/junk. A rite
  of passage; now you'll know the smell.
- **`c_char` vs `u8`** — a cast or type choice will be needed. Don't paper over
  it: *why* aren't C's `char` and Rust's `u8` the same type to the compiler?

## Level 2 (later, when this works): skip libc, make the syscall yourself

`gethostname` in libc is itself a thin wrapper: it loads a syscall number into a
register and runs the `syscall` instruction to ask the kernel directly. Once the
libc version works, the natural next step is to do that yourself — that's where
you actually touch **registers** and inline **`asm!`**. Don't do it now; it will
mean far more *after* you've felt the libc version.
