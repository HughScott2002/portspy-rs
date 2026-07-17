# Lesson 4 — Making it bulletproof: assertions

← [Lesson 3](./03-hostname-ffi.md) · [Index](./README.md) · Next: [Lesson 5 — Fuzzing](./05-fuzzing.md) →

**Goal:** add TigerStyle **assertions** so that if an assumption is ever violated,
the program stops **loudly, at the exact line**, instead of limping on and
corrupting something three functions away. You'll also *watch one fire* — because
seeing it is how it sticks.

**After this lesson you can:** use `assert!` / `debug_assert!` deliberately, and
you'll have hardened both your new FFI code and the web server.

Estimated loops: 3.

---

## 1. What an assertion is (and why it's TigerStyle's signature move)

An assertion is you writing a fact that must be true. If it's false, the program
**panics right there** with the file and line. It converts "silent wrong
behavior" into "loud, located failure" — which is the whole game in systems code,
where a wrong pointer or length might otherwise corrupt memory invisibly (you saw
this in the unsafe/UB discussion).

Two flavors:
- `assert!(cond)` — checked **always**, even in release. Use for cheap, important
  facts.
- `debug_assert!(cond)` — checked **only in debug builds**. Use for "obviously
  true" invariants you want a tripwire on during development, with zero release
  cost.

> 🧭 **Side quest (concept): why assert things that are "obviously" true?**
> Because *today's* obvious truth is *tomorrow's* broken assumption after someone
> edits nearby code. An assertion is a note to the future that fails fast the
> moment the truth stops holding. TigerStyle asserts liberally for exactly this.

### ✅ CHECKPOINT — watch one fire
Add this to any function you can call, temporarily:
```rust
assert!(false, "hello from a failed assertion");
```
Run `make print`. You'll see a panic naming the file and line. **That located,
loud failure is the entire point.** Delete the line.

---

## 2. Harden your Lesson 3 FFI code (do this first — it's freshest)

Your `hostname()` from Lesson 3 has assumptions worth pinning:

- Assert the buffer size const is greater than 0 (a zero-size buffer handed to C
  is a bug waiting to happen).
- After the `gethostname` call, you already check the return value — good; that's
  a *runtime* check for an *expected* failure. An assertion is different: it
  guards an *invariant you believe can never be false*. For example, before
  converting, you might assert the buffer contains a NUL within its bounds (C
  promised to terminate it) — if that's ever false, you want to know *now*, not
  read past the end.

> ⚠️ **Key distinction (important):** *validate* things the outside world can
> legitimately do wrong (a syscall returning `-1`) with normal `if`/return.
> *assert* things that must be true if your own logic is correct (buffer size
> positive, terminator present). Don't `assert!` on a syscall's success — that's
> a real runtime condition, not a broken invariant. Mixing these up is a classic
> mistake; getting it right is the skill.

### ✅ CHECKPOINT
```sh
cargo test -p portspy-proc && make print
```
Still green, still shows the hostname. Assertions that *don't* fire are invisible
— that's correct.

---

## 3. Harden the web server (the punch list)

The server (`crates/portspy-http/src/lib.rs`) uses the *bounds* half of TigerStyle
well but has **zero assertions**. Add them:

- `serve(address, web_dir)` — assert neither argument is empty.
- `respond(stream, status, content_type, body)` — assert `status` and
  `content_type` are non-empty (a blank status line is a malformed response we
  must never send).
- After the read in `handle_connection` — `debug_assert!(bytes_read <= buffer.len())`.
  Always true today, but if someone swaps the read call later, this catches it
  for free.

### The one real bug to fix (not just an assert)
`handle_connection` *assumes* the whole HTTP request arrives in a single `read`
(there's a comment admitting it). A slow client could split `GET /glue` and
`.js HTTP/1.1` across two packets, and the second half would be silently
mis-routed to `/`. TigerStyle: **assert your assumptions or handle them.** Pick:
- **Enforce:** after reading, check the buffer actually contains a full request
  line (a `\r\n`). If not, respond `400 Bad Request` instead of guessing.
- **Handle:** loop the read until you see `\r\n` or hit `MAX_REQUEST_BYTES`, then
  `400`. More correct, a little more code.

Either is fine for a local tool; the point is to stop *pretending* it always
holds.

### ✅ CHECKPOINT
```sh
make serve
```
In another terminal: `curl -s localhost:7878/api/ports | head -3` — still works.
Then try a deliberately empty request: `printf '' | nc -w1 localhost 7878` — your
server should handle it without panicking. `Ctrl-C` to stop.

> 🧭 **Side quest (tooling): what's `nc` / `curl` doing here?**
> `curl` makes a normal HTTP request; `nc` (netcat) opens a raw TCP connection so
> you can send *malformed* bytes on purpose — exactly the nasty input your new
> assertions and 400-handling defend against. This is manual fuzzing; Lesson 5
> automates it.

---

## 🎯 Done when
- [ ] You watched an assertion fire and understood the message.
- [ ] Your `hostname()` and the server have assertions at their boundaries.
- [ ] The single-read assumption is either enforced or handled (not just
      commented).
- [ ] `cargo test` and `make serve` still work.

## 🆘 Stuck?
- Panic in normal use after adding an assert? Your assert is wrong (too strict),
  or it caught a real bug — read the message and decide which. Both are wins.
- Not sure if something should be `assert!` or an `if` return? Re-read the "Key
  distinction" box. Rule of thumb: *could a correct outside world cause this?* If
  yes → `if`/return. If only *my* bug could → `assert!`.
- Release build behavior: note this repo uses `panic = "abort"`, so a failed
  `assert!` in a release build aborts the process immediately. That's intended.

## Commit it
```sh
git add -A && git commit -m "lesson 4: TigerStyle assertions on FFI + server; fix single-read assumption"
```

Next: [Lesson 5 — Breaking it on purpose: fuzzing](./05-fuzzing.md)
