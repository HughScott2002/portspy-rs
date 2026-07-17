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

In Lesson 3 you called `gethostname` from libc. But libc's `gethostname` is itself
a thin wrapper: it puts a **syscall number** in a CPU register, puts your buffer's
address in others, and runs one special instruction — `syscall` — that hands
control to the **kernel**. The kernel fills your buffer and returns. libc is just
a polite receptionist in front of that.

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

## 2. Stretch: re-implement `gethostname` with a raw syscall

Instead of calling libc, ask the kernel yourself. On Linux there's a syscall that
returns host info; the classic route is the `uname` syscall (number `63` on
x86-64), which fills a struct that includes the hostname (`nodename`).

This is where you finally touch **registers** and **inline assembly**. The shape,
conceptually:
- put the syscall number in the `rax` register,
- put the pointer to your struct in `rdi`,
- execute the `syscall` instruction,
- read the result back out of `rax`.

Rust exposes this via the `core::arch::asm!` macro. Your quest:

1. **Side quest (asm) first** — read Rust's inline-assembly chapter of the
   Reference (search "Rust inline assembly asm!") and skim the x86-64 syscall
   calling convention (which register holds what). This is where you meet
   registers hands-on for the first time — 30 minutes, and the `syscall` from §1
   will feel *far* less magic.
2. Define the `utsname` struct with `#[repr(C)]` — same idea as the pre-existing
   `Passwd` struct you read in `ffi.rs` back in Lesson 3: a Rust struct laid out
   to match C's memory layout exactly.
3. Write a tiny `unsafe` block using `asm!` to invoke syscall `63` on a pointer to
   your struct.
4. Pull `nodename` out and NUL-terminate it into a `String` — same skill as
   Lesson 3, Loop 5.

### ✅ CHECKPOINT (if you do the stretch)
Add it behind a new subcommand or a `--raw` flag, and compare:
```sh
./target/release/portspy print          # libc hostname
# your raw-syscall version                # should be identical
hostname                                  # the system's answer
```
All three match = you just talked to the kernel with no C library in between,
using assembly you wrote. That's about as deep as this stack goes from user space.

> ⚠️ Inline asm is `unsafe` and architecture-specific — this only works on
> x86-64 Linux. That's fine; the *point* is seeing the mechanism, not shipping it.
> And run it under **Valgrind** (Lesson 6) — raw memory + asm is exactly what it's
> for.

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
