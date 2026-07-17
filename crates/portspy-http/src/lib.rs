//! portspy-http
//!
//! A minimal HTTP/1.1 server, no dependencies, one connection at a time. It
//! exists to feed the browser dashboard. It answers exactly five requests:
//!
//!   GET /             -> the dashboard page          (web/index.html)
//!   GET /glue.js      -> the browser <-> wasm glue    (web/glue.js)
//!   GET /portspy.wasm -> the compiled wasm module     (web/portspy.wasm)
//!   GET /api/ports    -> the live port list, wire-encoded (fresh each request)
//!   anything else     -> 404
//!
//! Only these fixed paths are served. We never build a filesystem path out of
//! the request, so there is no way to ask for a file we did not intend.

use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;

/// Largest request we will read. We only need the first line (the request
/// line), so a small buffer is plenty and bounds our memory use.
const MAX_REQUEST_BYTES: usize = 8 * 1024;

/// Start serving forever. Returns only if binding the address fails.
///
/// `web_dir` is the folder holding index.html / glue.js / portspy.wasm.
pub fn serve(address: &str, web_dir: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(address)?;

    println!("Portspy serving on http://{address}");
    println!("Open that in your browser (Ctrl-C to stop)");

    // Handle connections one at a time. Simple and perfectly fine for a local
    // single-user tool.
    for incoming in listener.incoming() {
        let stream = match incoming {
            Ok(stream) => stream,
            Err(_) => continue, // drop a bad connection, keep serving
        };

        // One misbehaving request should never take the server down, so we
        // handle its errors here instead of propagating them.
        if let Err(error) = handle_connection(stream, web_dir) {
            eprintln!("connection error: {error}");
        }
    }

    Ok(())
}

/// Read one request, decide what it wants, and write one response.
fn handle_connection(mut stream: TcpStream, web_dir: &str) -> std::io::Result<()> {
    // Read up to MAX_REQUEST_BYTES. We do a single read: for these tiny GET
    // requests the whole request line arrives in the first packet.
    let mut buffer = [0u8; MAX_REQUEST_BYTES];
    let bytes_read = stream.read(&mut buffer)?;
    let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

    // The path lives in the first line: "GET /some/path HTTP/1.1".
    let path = request_path(&request_text);

    // Route on the exact path. Each arm produces (content_type, body_bytes).
    match path.as_str() {
        "/" => {
            let body = read_web_file(web_dir, "index.html");
            respond(&mut stream, "200 OK", "text/html; charset=utf-8", &body)
        }
        "/glue.js" => {
            let body = read_web_file(web_dir, "glue.js");
            respond(
                &mut stream,
                "200 OK",
                "text/javascript; charset=utf-8",
                &body,
            )
        }
        "/portspy.wasm" => {
            let body = read_web_file(web_dir, "portspy.wasm");
            respond(&mut stream, "200 OK", "application/wasm", &body)
        }
        "/api/ports" => {
            // Fresh data on every request: read the OS right now, encode it.
            let records = portspy_proc::collect();
            let body = portspy_model::wire::encode(&records);
            respond(
                &mut stream,
                "200 OK",
                "text/plain; charset=utf-8",
                body.as_bytes(),
            )
        }
        _ => {
            let body = b"not found";
            respond(
                &mut stream,
                "404 Not Found",
                "text/plain; charset=utf-8",
                body,
            )
        }
    }
}

/// Pull the path out of an HTTP request's first line.
///
/// "GET /glue.js HTTP/1.1\r\n..."  ->  "/glue.js". Falls back to "/" if the
/// request is malformed.
fn request_path(request_text: &str) -> String {
    // First line only.
    let first_line = match request_text.lines().next() {
        Some(line) => line,
        None => return String::from("/"),
    };

    // Split "METHOD PATH VERSION" on spaces; the path is the middle token.
    let mut parts = first_line.split(' ');
    let _method = parts.next();
    match parts.next() {
        Some(path) => path.to_string(),
        None => String::from("/"),
    }
}

/// Read one of the fixed dashboard files from `web_dir`.
///
/// The file name is a hard-coded constant chosen by the caller, never anything
/// from the request, so this cannot be tricked into reading elsewhere.
fn read_web_file(web_dir: &str, file_name: &str) -> Vec<u8> {
    let path = Path::new(web_dir).join(file_name);
    match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => {
            let message = format!("missing file: {}", path.display());
            message.into_bytes()
        }
    }
}

/// Write a complete HTTP response: status line, headers, blank line, body.
fn respond(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    // Build the header block first, then send headers and body.
    let header = format!(
        "HTTP/1.1 {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\
         \r\n",
        len = body.len(),
    );

    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}
