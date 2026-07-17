# Feature: harden the server with TigerStyle assertions

> Task doc. The server (`crates/portspy-http/src/lib.rs`) already uses the
> *bounds* and *explicit-flow* halves of TigerStyle, but it has **zero
> assertions** — TigerStyle's signature move. This is the punch list to fix that.
> Do it as its own small commit; it's a great way to *see* the style.

## The idea in one line

An assertion is you writing down a fact that must be true, so that if a future
change makes it false, the program **stops loudly right there** instead of
limping on and corrupting something three functions away. Assert your
arguments, your invariants, and your assumptions.

Two flavours:
- `assert!(...)` — always checked, even in release. Use for cheap, important
  facts.
- `debug_assert!(...)` — checked only in debug builds. Use for facts that are
  "obviously" true but you want a tripwire on during development.

## The punch list (in `portspy-http/src/lib.rs`)

### 1. Assert the arguments at each boundary
- `serve(address, web_dir)`: assert neither is empty.
- `respond(stream, status, content_type, body)`: assert `status` and
  `content_type` are non-empty. A blank status line is a malformed response we
  should never send.
- `read_web_file(web_dir, file_name)`: assert `file_name` is non-empty.

### 2. Assert the "obvious" invariants (assert them *because* they're obvious)
- After the read: `bytes_read <= buffer.len()`. Always true today — but if
  someone later swaps the read call, this tripwire catches it for free.
- In `respond`, the header you build should end in a blank line (`\r\n\r\n`).
  A `debug_assert!` on that shape documents and enforces the contract.

### 3. Turn the documented *assumption* into an enforced one
Today `handle_connection` assumes the whole request arrives in one `read`
(there's a comment saying so). TigerStyle says: assert your assumptions or
handle them. Two honest options — pick one:
- **Enforce it:** after the read, check the buffer actually contains a full
  request line (a `\r\n`). If not, respond `400 Bad Request` instead of silently
  mis-routing a split request to `/`.
- **Handle it:** loop the read until you've seen `\r\n` or hit
  `MAX_REQUEST_BYTES` (then `400`). More correct, slightly more code.

Either is fine for a local tool; the point is to stop *pretending* the
assumption always holds.

## Pattern to copy from TigerStyle: assert both sides

TigerStyle likes asserting the *positive and negative space* — not just "x is
valid" but also "x is not the thing it must never be." Example for the router:
after computing `path`, you could assert it starts with `/` (a well-formed HTTP
target always does), which documents the shape the `match` relies on.

## How you'll know it worked

Assertions that fire show up as a **panic** with the file and line:
```
thread 'main' panicked at crates/portspy-http/src/lib.rs:NN:
assertion failed: !address.is_empty()
```
To *see* one fire on purpose, temporarily call `serve("", "web")` — you should
get an immediate, precise panic instead of a confusing failure later. Put it
back afterward.

Note: release builds here use `panic = "abort"`, so a failed `assert!` in
release aborts the process (no unwinding). That's the intended TigerStyle
behaviour: a broken invariant should stop the program, not be caught and
ignored.

## Scope

Just `portspy-http` for this pass. Once it feels natural, the same treatment
belongs in `portspy-proc` (assert parsed ports/inodes are in range) and the
`portspy-wasm` boundary (assert pointers/lengths before use).
