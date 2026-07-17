# Lesson 1 — Orientation: run it, tour it, watch it call C

← [Course index](./README.md) · Next: [Lesson 2 — Testing](./02-testing.md) →

**Goal:** get portspy running in front of you, understand the pieces, and *see*
the single most important systems-programming fact — that your program is calling
code from a library that's already loaded in memory.

**After this lesson you can:** run portspy two ways, say what each crate does, and
explain what `U getpwuid` means when you see it.

Estimated loops: 4 (each ends in something on your screen).

---

## 1. Run the thing

### ✅ CHECKPOINT — the terminal view
```sh
make print
```
You should see a table: PROTO, ADDRESS, PORT, PID, PROCESS, USER, COMMAND. Those
are the real sockets your machine is listening on *right now*.

### ✅ CHECKPOINT — the web view
```sh
make serve
```
Open <http://127.0.0.1:7878>. Same data, in a browser. Type in the search box —
the filtering is running in **WebAssembly** (Rust compiled to run in the
browser). Press `Ctrl-C` in the terminal to stop it.

You just ran the *same Rust logic* two ways: as a native Linux binary, and as
wasm in a browser. Hold that thought — it comes back in Lesson 3 and 5.

---

## 2. Tour the map

portspy is a **Cargo workspace**: several small crates (libraries), each with one
job, that combine into one binary. Open the folders as you read.

```
crates/
  portspy-model/   the shared "vocabulary": data types + the text format +
                   sort/filter. Pure logic, no I/O. Compiles for BOTH native
                   and wasm — it's the piece both worlds share.
  portspy-proc/    reads the operating system: parses /proc, calls into C.
                   (native only)
  portspy-http/    a tiny web server, no dependencies. (native only)
  portspy-cli/     the actual `portspy` binary; owns the terminal table.
  portspy-wasm/    the same model logic, compiled for the browser.
```

Data flows like this:

```
/proc + libc  ──▶  portspy-proc  ──▶  Record  ──▶  portspy-cli (table)
                                          └──────▶  portspy-http ──▶ browser ──▶ portspy-wasm
```

> 🧭 **Side quest (Rust): what's a "crate" and a "workspace"?**
> A *crate* is Rust's unit of compilation — roughly "one library or one binary."
> A *workspace* is a folder of crates that share one build and one
> `Cargo.lock` (like a monorepo of packages sharing one `package-lock.json`).
> Open the top-level `Cargo.toml` — the `members = [...]` list *is* the map
> above. **2-minute detour, then come back.**

### ✅ CHECKPOINT — find the C
Open `crates/portspy-proc/src/ffi.rs`. That's the **only** file in the whole
project that talks to C directly. Skim it. See the `unsafe extern "C"` block with
`getpwuid` and `geteuid`? Those are functions portspy *calls* but does not
*contain*. Where do they come from? That's the rest of this lesson.

---

## 3. The big idea: "systems programming" = talking to the layers below you

In TypeScript you sit on top of a huge stack (V8, Node, the OS) and rarely touch
it. Systems programming is working *at* those lower layers: asking the **kernel**
for information, managing **memory** yourself, and calling **C libraries** that
the whole system shares.

portspy does all three: it reads kernel data (`/proc`), it hands raw memory
buffers around (the wasm boundary), and it calls a C library (libc) to turn a
user id number into a name. Let's make that last one concrete.

---

## 4. Watch your program call a library that's already loaded

Your `getpwuid` call works because a C library — **libc** — is already loaded
into essentially every process on a Linux machine, yours included. You don't open
it or import it; you just point at code that's already in memory. Let's prove it.

### ✅ CHECKPOINT — what libraries does portspy load?
```sh
ldd target/release/portspy
```
You'll see a few lines including `libc.so.6 => /lib/.../libc.so.6`. That's the C
library, mapped into your program at runtime.

### ✅ CHECKPOINT — the hole and the fill
```sh
# symbols your binary NEEDS but does not contain (U = undefined):
nm -D --undefined-only target/release/portspy | grep -E 'getpwuid|geteuid'

# the same symbols, DEFINED inside libc (T = real code lives here):
LIBC=$(ldd target/release/portspy | awk '/libc.so/{print $3}')
nm -D "$LIBC" | grep -Ew 'getpwuid|geteuid'
```
Your binary shows `U getpwuid` — a **hole**, "I call this but don't have it."
libc shows `T getpwuid` at some address — the **real machine code**. At startup,
the dynamic linker patches your hole to point at libc's address. After that,
calling `getpwuid` is just a jump to that address.

> 🧭 **Side quest (C/OS): what's a "shared library" / "dynamic linking"?**
> A shared library (`.so` on Linux, `.dll` on Windows, `.dylib` on macOS) is
> compiled code that many programs share *one copy of* in memory. "Linking" is
> the step that connects your `U` holes to someone's `T` definitions — it can
> happen at build time (static) or at startup (dynamic, what you just saw).
> **The one-line model:** *`extern` leaves a labeled hole; the linker wires that
> hole to code already loaded in memory.* If you want the deep version, this is
> Chapter 7 of *Computer Systems: A Programmer's Perspective* — but the one-liner
> is enough to continue.

---

## 🎯 Done when
- [ ] `make print` shows a table and `make serve` shows the dashboard.
- [ ] You can name what each of the five crates does.
- [ ] You ran `ldd` and `nm` and can explain, in your own words, what
      `U getpwuid` vs `T getpwuid` means.

## 🆘 Stuck?
- `ldd`/`nm` not found? They're in `binutils` — `sudo apt install binutils`.
- `nm` prints nothing for libc? Your libc path may differ; run
  `ldd target/release/portspy` and use whatever path it prints for `libc.so`.
- Table is nearly empty? That's fine — you may just have few listeners. Start one
  to see more: `python3 -m http.server 8000` in another terminal, then re-run.

## Commit it
```sh
git commit --allow-empty -m "lesson 1: oriented — ran portspy, saw libc linked in"
```
(Empty commit is fine here — you didn't change code, but it marks your progress.)

Next: [Lesson 2 — Your safety net: testing](./02-testing.md)
