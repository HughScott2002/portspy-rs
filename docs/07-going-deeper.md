# Lesson 7 — Going deeper: syscalls, and where next

← [Lesson 6](./06-memory-checkers.md) · [Index](./README.md) · 🎓 Final lesson

**Goal:** peek one layer below libc — make the kernel call *yourself*, touching
registers and inline assembly — then set you up with a map for the rest of the
journey.

**After this lesson you can:** explain what a syscall really is, and (optionally)
you'll have re-implemented `gethostname` without libc at all.

This lesson is a **stretch goal + a springboard.** The stretch is genuinely
advanced; do it if you're enjoying yourself, skip it without guilt if not — the
reading map at the bottom matters more.

---

## 1. What libc was actually doing

In Lesson 3 you called `gethostname` from libc. Here's the twist: on Linux
**there is no `gethostname` syscall.** libc's `gethostname` is a *library
function* built on top of a different syscall — `uname`. When you call it, glibc
puts the `uname` **syscall number** in a CPU register, puts a pointer to a
`utsname` struct in another, runs one special instruction — `syscall` — which
hands control to the **kernel**; the kernel fills the whole struct, and then glibc
copies just the `nodename` field into your buffer. libc is a polite receptionist
in front of that `syscall` instruction.

So the stretch below doesn't re-create a "gethostname syscall" (there isn't one) —
it re-creates the `uname` call that libc was making for you.

> 🧭 **Side quest (concept): what *is* a system call?**
> Your program runs in "user mode" and can't touch hardware or other processes
> directly — the kernel does that, in "kernel mode." A **syscall** is the
> doorbell: a controlled switch into the kernel to ask for a privileged thing
> (read a file, get the hostname, open a socket). Everything your code does that
> affects the outside world bottoms out in a syscall. `strace ./target/release/portspy print`
> prints every syscall portspy makes — run it and scroll; you'll see the `/proc`
> reads fly by. **That single command is a great "oh, *that's* what's happening"
> moment.**

### ✅ CHECKPOINT — see the syscalls
```sh
strace -f ./target/release/portspy print 2>&1 | grep -E 'openat|read' | head
```
Those `openat("/proc/...")` lines are portspy talking to the kernel. (No
`strace`? `sudo apt install strace`.)

---

Instead of calling libc, get the hostname from the kernel's `uname` — first the
safe way, then (if you're brave) the raw-assembly way.

Both need the same struct. On Linux, `struct utsname` is **six fixed 65-byte
`char` arrays** (`sysname`, `nodename`, `release`, `version`, `machine`, and a
domainname field), one after another. Define it with `#[repr(C)]` — same idea as
the pre-existing `Passwd` struct you read in `ffi.rs` in Lesson 3: a Rust struct
laid out to match C's memory exactly. `nodename` is the hostname.

### Step A (recommended) — call the `uname` *libc function* via FFI
This is the honest "one layer down" without any assembly, and it's testable:

1. Add `fn uname(buf: *mut Utsname) -> c_int;` to your `unsafe extern "C"` block
   (same skill as `gethostname` in Lesson 3).
2. Zero-initialize a `Utsname`, call `uname(&mut buf)` inside `unsafe`, check the
   return (`0` = ok).
3. Read `nodename`, stop at the first NUL, make a `String` (Lesson 3, Loop 5).

That already teaches the `#[repr(C)]` struct + FFI. If you stop here, you've won.

### Step B (advanced) — do the `syscall` yourself with `asm!`
This is where you finally touch **registers** and **inline assembly** via
`core::arch::asm!`. It is genuinely tricky, so treat it as *guided reading you
then attempt*, not copy-paste. The x86-64 Linux ABI has rules you **must** honor
or the compiler will miscompile silently:

- syscall **number 63** goes in `rax`; the struct pointer goes in `rdi`; run the
  `syscall` instruction; the result comes back in `rax`.
- the `syscall` instruction **destroys `rcx` and `r11`** — you must tell Rust with
  `lateout("rcx") _` and `lateout("r11") _`, or it will assume those registers
  survived and generate wrong code.
- on error the kernel returns a **negative errno in `rax`** (e.g. `-14`), *not*
  libc's `-1`-plus-`errno`. Check for negative.
- zero-initialize the struct before the call.

Read Rust's inline-assembly chapter of the Reference (search "Rust inline
assembly asm!") and the x86-64 syscall calling convention before writing a line.
Gate the whole thing behind `#[cfg(target_arch = "x86_64")]` — it only works
there.

> This is the one place in the course where, if it feels like too much, **the
> right move is Step A.** Nobody ships hand-rolled syscall asm for a hostname; the
> point is to *see the mechanism* once.

### ✅ CHECKPOINT
Wire your new function into a `hostname` subcommand in `main.rs` (like `print` /
`serve`), so you have a concrete thing to run. Then **rebuild** (don't trust the
old binary) and compare against the system's own answer:
```sh
cargo build --release -p portspy-cli
./target/release/portspy hostname     # your uname-based value
hostname                              # the system's answer
```
They match = you got the hostname straight from the kernel's `uname`, with (Step
B) assembly you wrote and no C library in the middle. That's about as deep as this
stack goes from user space.

> ⚠️ Step B's inline asm is `unsafe` and x86-64-only. Run it under **Valgrind**
> (Lesson 6) — raw memory + a hand-written syscall is exactly what Valgrind is
> for — and gate it with `#[cfg(target_arch = "x86_64")]`.

> 🆘 **Too much?** Totally reasonable to stop before the asm. An easier middle
> step: call the `uname` **libc** function (no asm) via FFI — same struct, same
> `nodename`, but libc does the syscall for you. That still teaches the struct +
> `#[repr(C)]` layout without the assembly. Do that instead and call it a win.

---

## 3. What you've actually learned

Step back. Across seven lessons you touched the real foundations:

- **Linking** — holes and definitions, dynamic libraries (L1).
- **Testing** — fast feedback, pure-function tests, properties (L2).
- **FFI & `unsafe`** — calling C, owning buffers, the cost of `unsafe` (L3).
- **Assertions & invariants** — failing loud and located (L4).
- **Fuzzing** — adversarial, coverage-guided input (L5).
- **Memory safety tooling** — UB, Valgrind, Miri (L6).
- **Syscalls & the user/kernel boundary** — where it all bottoms out (L7).

These aren't Rust trivia. They're the layer under *every* language you'll use —
Node, Python, Go, whatever's next. You now know what's beneath the abstraction.

---

## 4. Where to go next (the reading map)

**One anchor book:** *Computer Systems: A Programmer's Perspective* (Bryant &
O'Hallaron, "CS:APP"). Connects registers → assembly → linking → syscalls in one
narrative. If you read one thing, read Ch. 3 and Ch. 7.

**By thread:**
- **C itself:** *C Programming: A Modern Approach* (K.N. King); **Beej's Guide to
  C** (free, friendly).
- **FFI / unsafe Rust:** *The Rustonomicon* (free, official); *Rust for
  Rustaceans* (Jon Gjengset).
- **Linking:** CS:APP Ch. 7; Ian Lance Taylor's linker essay (free online).
- **Syscalls / asm:** *Programming from the Ground Up* (Bartlett, free); *The
  Linux Programming Interface* (Kerrisk) as a reference.
- **WebAssembly:** the official *Rust and WebAssembly* book; MDN's WASM docs.

**YouTube:** Ben Eater (build a CPU — makes registers physical), Jon Gjengset
("Crust of Rust" — deep Rust incl. unsafe/FFI), Low Level Learning (bite-size C /
systems), Computerphile (concept explainers).

**Extend portspy (to keep practicing):**
- add UDP (`/proc/net/udp`) — a near-copy of the TCP parser you now understand.
- add a `--json` output mode (write a tiny serializer in `portspy-model`).
- show each process's start time or memory (more `/proc` reading + FFI).

Each of those re-uses everything you built here, with a little new surface — the
ideal way to make it stick.

---

## 🎓 Done when
- [ ] You ran `strace` and recognized portspy's syscalls.
- [ ] You either did the raw-syscall stretch, the `uname`-via-libc middle step, or
      consciously chose to skip both.
- [ ] You picked *one* next resource and one portspy extension to try.

## Commit it
```sh
git add -A && git commit -m "lesson 7: strace + (optional) raw syscall hostname; course complete"
```

🎉 **That's the course.** You started at "how do I even call a library?" and
ended at "I can talk to the kernel in assembly." Small scope, real foundations.
Go build something.

← Back to the [Index](./README.md)
