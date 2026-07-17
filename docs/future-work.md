# Future work

Ideas parked for later. Not built yet.

Related task/learning docs:
- [tigerstyle-assertions.md](./tigerstyle-assertions.md) — harden the server
  with assertions.
- [testing-and-fuzzing.md](./testing-and-fuzzing.md) — learn `cargo test` and
  `cargo-fuzz` against portspy's parsers.
- [hostname-feature.md](./hostname-feature.md) — the guided FFI exercise.

## Memory-checking Make targets (`make valgrind` / `make miri`)

When we write `unsafe`, we switch off Rust's built-in referee. These two tools
hand a (slower, optional) referee back for the moments we need one. The plan is
to make each runnable with a single command.

### Why two tools

Think of memory as a wall of numbered lockers. `unsafe` lets us reach for any
locker by number, and normally nobody checks. These watch for us:

- **Valgrind** — a referee watching the *real* program run. Catches reads/writes
  to lockers that aren't ours, use-after-free, and leaks, and blows the whistle
  with the exact line. Works **through FFI into libc**, so it's the one for
  `portspy-proc` (getpwuid / gethostname). Slower, but no code changes needed.
- **Miri** — a robot that *simulates* our Rust line by line against a strict
  rulebook. Catches even sneaky stuff (e.g. reading uninitialized memory). But
  it can't make "real phone calls," so it **cannot run our libc calls** — it's
  for the *pure-Rust* unsafe in `portspy-wasm` (the alloc / dealloc / pointer
  packing).

Rule of thumb: **Miri for pure-Rust unsafe, Valgrind for FFI unsafe.**

### What "done" looks like

Two new Makefile targets:

- `make valgrind` — build the debug binary and run it under Valgrind, e.g.
  `valgrind ./target/debug/portspy print`. Silent output = clean. Any
  "Invalid read/write" = a real bug to fix.
- `make miri` — run Miri against the wasm crate's pure-Rust unsafe, e.g.
  `cargo +nightly miri test -p portspy-wasm` (add a small test that exercises
  `alloc` / `process` / `dealloc` so Miri has something to simulate).

### Prerequisites to note in the target / docs

- Valgrind: install the system package (`valgrind`).
- Miri: nightly toolchain + component — `rustup +nightly component add miri`.
- Reminder: Miri will refuse to run anything that calls a foreign function, so
  keep its scope to `portspy-wasm`. Point it at `portspy-proc` and it will error
  on the first libc call — that's expected, not a setup problem.

### Good first trigger

Run both right after finishing the `gethostname` feature (see
[hostname-feature.md](./hostname-feature.md)) — it's the newest `unsafe`, and
the buffer round-trip is exactly the kind of thing Valgrind catches if the
length is wrong.
