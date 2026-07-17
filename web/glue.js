// glue.js — the ~100 lines that let plain JavaScript talk to the Rust wasm
// module. It does no sorting or filtering itself; it hands the raw port list to
// wasm and renders whatever comes back. All the logic lives in Rust.
//
// The conversation with wasm always follows the same four steps:
//   1. alloc a buffer in wasm memory
//   2. copy our bytes into it
//   3. call a wasm function
//   4. read the result out, then free the buffers
//
// See crates/portspy-wasm/src/lib.rs for the other side of this boundary.

// ---- state -----------------------------------------------------------------

let wasm = null; // the wasm module's exported functions
let rawPortText = ""; // the unmodified /api/ports response (source of truth)
let currentSortKey = 0; // 0 = port (see SortKey in Rust)

const encoder = new TextEncoder();
const decoder = new TextDecoder();

// ---- boot ------------------------------------------------------------------

async function boot() {
  // Instantiate the wasm module. It needs no imports from us.
  const response = await fetch("/portspy.wasm");
  const result = await WebAssembly.instantiateStreaming(response, {});
  wasm = result.instance.exports;

  wireUpControls();
  await refresh();
}

// ---- talking to wasm -------------------------------------------------------

// Copy a JS string into a fresh wasm buffer. Returns { ptr, len } so the caller
// can pass them to a wasm function and free them afterward.
function copyStringIntoWasm(text) {
  const bytes = encoder.encode(text);
  const ptr = wasm.alloc(bytes.length);

  // A view over wasm's memory at the buffer we just got, and write into it.
  const memory = new Uint8Array(wasm.memory.buffer, ptr, bytes.length);
  memory.set(bytes);

  return { ptr, len: bytes.length };
}

// Read `len` bytes out of wasm memory at `ptr` and decode them as a string.
// We copy (.slice) because wasm memory can move out from under us later.
function readStringFromWasm(ptr, len) {
  const bytes = new Uint8Array(wasm.memory.buffer, ptr, len).slice();
  return decoder.decode(bytes);
}

// Run the raw port text through the wasm sort+filter and return the result text.
function runProcess(portText, sortKey, needle) {
  // Step 1+2: put both inputs into wasm memory.
  const input = copyStringIntoWasm(portText);
  const filter = copyStringIntoWasm(needle);

  // Step 3: call into Rust. It returns a 64-bit number (a BigInt in JS) that
  // packs the answer's length in the high 32 bits and its pointer in the low.
  const packed = wasm.process(
    input.ptr,
    input.len,
    sortKey,
    filter.ptr,
    filter.len,
  );
  const outLen = Number(packed >> 32n);
  const outPtr = Number(packed & 0xffffffffn);

  // Step 4: read the answer, then free every buffer (ours and Rust's).
  const outText = readStringFromWasm(outPtr, outLen);
  wasm.dealloc(input.ptr, input.len);
  wasm.dealloc(filter.ptr, filter.len);
  wasm.dealloc(outPtr, outLen);

  return outText;
}

// ---- data + rendering ------------------------------------------------------

// Fetch the live port list from the native agent and re-render.
async function refresh() {
  const response = await fetch("/api/ports");
  rawPortText = await response.text();
  render();
}

// Ask wasm to shape the current data for the current search + sort, then draw.
function render() {
  const needle = document.getElementById("search").value;
  const shapedText = runProcess(rawPortText, currentSortKey, needle);
  const records = parseWireText(shapedText);
  drawRows(records);
}

// Parse the wire format (tab-separated fields, newline-separated records) that
// both Rust sides use. Field order matches wire.rs.
function parseWireText(text) {
  const records = [];
  if (text.length === 0) return records;

  for (const line of text.split("\n")) {
    if (line.length === 0) continue;
    const f = line.split("\t");
    if (f.length !== 8) continue;
    records.push({
      proto: f[0],
      addr: f[1],
      port: f[2],
      state: f[3],
      pid: f[4],
      process: f[5],
      user: f[6],
      cmd: f[7],
    });
  }
  return records;
}

// Build the table body from records.
function drawRows(records) {
  const tbody = document.getElementById("rows");
  tbody.innerHTML = "";

  for (const r of records) {
    const tr = document.createElement("tr");
    tr.appendChild(cell(r.port));
    tr.appendChild(cell(r.process || "-"));
    tr.appendChild(cell(r.pid === "0" ? "-" : r.pid));
    tr.appendChild(cell(r.user || "-"));
    tr.appendChild(cell(r.proto));
    tr.appendChild(cell(r.addr));

    const cmd = cell(r.cmd || "-");
    cmd.className = "cmd";
    cmd.title = r.cmd;
    tr.appendChild(cmd);

    // Clicking a row copies the kill command for that pid.
    tr.addEventListener("click", () => copyKill(r.pid));
    tbody.appendChild(tr);
  }
}

function cell(text) {
  const td = document.createElement("td");
  td.textContent = text;
  return td;
}

// Copy "kill <pid>" to the clipboard and flash a confirmation in the hint line.
function copyKill(pid) {
  const hint = document.getElementById("hint");
  if (pid === "0" || pid === "") {
    hint.textContent = "no known pid for that port (try: sudo portspy serve)";
    return;
  }

  const command = "kill " + pid;
  navigator.clipboard.writeText(command).then(
    () => {
      hint.innerHTML =
        'copied <span class="flash">' + command + "</span> to clipboard";
    },
    () => {
      hint.textContent = "run: " + command;
    },
  );
}

// ---- controls --------------------------------------------------------------

function wireUpControls() {
  // Live filtering: re-render on every keystroke (wasm does the actual work).
  document.getElementById("search").addEventListener("input", render);

  // Manual refresh re-reads the OS.
  document.getElementById("refresh").addEventListener("click", refresh);

  // Clicking a header with a data-key sorts by that column.
  const headers = document.querySelectorAll("th[data-key]");
  for (const th of headers) {
    th.addEventListener("click", () => {
      currentSortKey = Number(th.getAttribute("data-key"));
      render();
    });
  }
}

boot();
