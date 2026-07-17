# portspy: a hands-on systems programming mini-course

A short, self-contained course for a **TypeScript developer** who wants real
**systems programming** foundations — linking, calling C, memory safety, testing,
fuzzing — without a huge time sink. It's built around one small real program you
already have in this repo: **portspy** (it lists which processes are listening on
your TCP ports).

Think **Rustlings, but as docs**: read a lesson, implement one piece, **run it,
see it happen**, commit, move on.

## Who this is for

- You're comfortable in TypeScript.
- C, pointers, linking, syscalls, and "unsafe" are new or fuzzy.
- You want *foundations that last a career*, but in a **small, finishable scope**
  — properly **tested, safe, and bulletproof**, not a toy you abandon.

## The rules of the game (read these — they're the whole method)

1. **Every lesson has `✅ CHECKPOINT`s.** Run the command, see the output. Do
   **not** move to the next section until you've seen the expected thing on your
   screen. Momentum comes from stuff visibly happening.
2. **Do the `🧭 Side quests`.** When a lesson hits a C/systems idea you don't
   have yet, it sends you on a tiny detour to learn just that thing. That *is*
   the learning — not a distraction from it.
3. **Fake it, then make it.** When you can, put a hardcoded placeholder on screen
   *first* so you see the shape working, then replace it with the real thing.
   Short loop beats correct-but-invisible.
4. **Commit after every lesson.** It's a save point and a little hit of done.
   Each lesson ends with a suggested commit message.
5. **Stuck > 15 min?** Every lesson has a `🆘 Stuck?` section. Use it. Then keep
   going.

## The north star: TigerStyle

Everything you write in this course follows **TigerStyle** — a safety-first way
of writing systems code. In one breath:

- **Put a limit on everything** (named constants, fixed-size buffers, bounded
  loops). Nothing unbounded from the outside world.
- **Assert your assumptions** so a broken one stops the program *loudly* instead
  of corrupting something silently.
- **One job per function**, explicit control flow, no clever one-liners.
- **Test it. Fuzz it. Check it for memory bugs.** "It ran once" is not "it
  works."

You'll practice each of these in its own lesson.

## What you'll have at the end

A small, real, **tested + assertion-hardened + fuzzed + memory-checked** systems
tool — and hands-on experience with the ideas under every language you'll ever
use: how a program calls a library, how it talks to the kernel, what "unsafe"
really costs, and how to prove your code is solid.

## The map

| # | Lesson | You'll do / see |
|---|--------|-----------------|
| 1 | [Orientation: run it, tour it, watch it call C](./01-orientation.md) | Run portspy; prove a C library is already loaded in your process |
| 2 | [Your safety net: testing](./02-testing.md) | Write `cargo test`s on the existing code; get green fast |
| 3 | [Calling C yourself: the hostname feature](./03-hostname-ffi.md) | Hand-write a libc FFI call; watch the hostname appear |
| 4 | [Making it bulletproof: assertions](./04-assertions.md) | Add TigerStyle `assert!`s; watch one fire on purpose |
| 5 | [Breaking it on purpose: fuzzing](./05-fuzzing.md) | Let a robot try to crash your parsers; turn a crash into a test |
| 6 | [Did I corrupt anything? Valgrind & Miri](./06-memory-checkers.md) | Prove your `unsafe` didn't scribble on memory |
| 7 | [Going deeper: syscalls, and where next](./07-going-deeper.md) | Skip libc and hit the kernel directly (registers + asm); reading list |

Work them **in order** — each builds on the one before.

## ✅ CHECKPOINT 0 — setup

Before Lesson 1, confirm your toolbox. Run these; you want output like the
comments:

```sh
rustc --version     # rustc 1.9x  (any recent stable is fine)
cargo --version     # cargo 1.9x
make --version      # GNU Make ...
```

Then install the one extra compile target the browser dashboard needs (once),
and prove the app builds and runs:

```sh
make setup     # one-time: installs the wasm32 target used by `make serve`
make print
```

You should see a table of listening ports. **If you see that table, you're
ready.** Head to [Lesson 1](./01-orientation.md).

> 🆘 **Stuck?** `make print` failing? Run `cargo build --release -p portspy-cli`
> and read the first error. Missing `make`? You can run the underlying commands
> directly — every lesson shows them. `rustc` missing entirely? Install Rust via
> <https://rustup.rs>.
