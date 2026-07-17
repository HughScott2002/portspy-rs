//! portspy - see what is listening on your ports, and who owns it.
//!
//! Usage:
//!   portspy            list listening ports as a table (default)
//!   portspy print      same as above, explicitly
//!   portspy serve      start the web dashboard on 127.0.0.1:7878
//!   portspy serve 9000 start the web dashboard on a chosen port
//!   portspy help       show this help
//!
//! This file owns the terminal experience only. The actual OS reading lives in
//! portspy-proc, and the web server in portspy-http.

use portspy_model::Record;
use portspy_model::SortKey;
use portspy_model::view;

/// Default port for the web dashboard, used when `serve` gets no number.
const DEFAULT_SERVE_PORT: u16 = 7878;

/// Folder that holds the dashboard's static files.
const WEB_DIR: &str = "web";

fn main() {
    // args[0] is the program name; the command (if any) is args[1].
    let args: Vec<String> = std::env::args().collect();
    let command = match args.get(1) {
        Some(text) => text.as_str(),
        None => "print", // no command given -> default to printing
    };

    match command {
        "print" => run_print(),
        "serve" => run_serve(&args),
        "help" | "-h" | "--help" => print_help(),
        other => {
            eprintln!("portspy: unknown command '{other}'\n");
            print_help();
        }
    }
}

/// The default action: collect the ports and print them as a table.
fn run_print() {
    // 1. read the OS.
    let mut records = portspy_proc::collect();

    // 2. sort by port so the table is stable and easy to scan.
    view::sort(&mut records, SortKey::Port);

    // 3. draw it.
    print_table(&records);

    // 4. if we are not root, some rows may be missing their owner; say so once.
    if !portspy_proc::is_root() {
        let any_unknown = records.iter().any(|record| record.pid == 0);
        if any_unknown {
            println!();
            println!("note: some ports have no owner shown. Re-run with sudo to see");
            println!("      processes owned by other users:  sudo portspy");
        }
    }
}

/// The `serve` action: parse an optional port, then start the web server.
fn run_serve(args: &[String]) {
    // An optional port may follow "serve": portspy serve 9000
    let port = match args.get(2) {
        Some(text) => match text.parse::<u16>() {
            Ok(value) => value,
            Err(_) => {
                eprintln!("portspy: '{text}' is not a valid port number");
                return;
            }
        },
        None => DEFAULT_SERVE_PORT,
    };

    let address = format!("127.0.0.1:{port}");

    // serve() only returns if it fails to bind the address.
    if let Err(error) = portspy_http::serve(&address, WEB_DIR) {
        eprintln!("portspy: could not start server on {address}: {error}");
    }
}

/// Print the records as an aligned ASCII table.
///
/// We compute each column's width first (the widest value in it), then print
/// the header and every row padded to those widths. Explicit and predictable.
fn print_table(records: &[Record]) {
    if records.is_empty() {
        println!("no listening TCP ports found.");
        return;
    }

    // Column headers, in display order.
    let headers = [
        "PROTO", "ADDRESS", "PORT", "PID", "PROCESS", "USER", "COMMAND",
    ];

    // Turn every record into an array of already-formatted cell strings, so the
    // width math and the printing both work on the same text.
    let mut rows: Vec<[String; 7]> = Vec::new();
    for record in records {
        rows.push([
            record.proto.clone(),
            record.addr.clone(),
            record.port.to_string(),
            format_pid(record.pid),
            non_empty(&record.process),
            non_empty(&record.user),
            non_empty(&record.cmd),
        ]);
    }

    // Find the widest cell in each column, starting from the header width.
    let mut widths = [0usize; 7];
    for column in 0..7 {
        widths[column] = headers[column].len();
    }
    for row in &rows {
        for column in 0..7 {
            let cell_len = row[column].chars().count();
            if cell_len > widths[column] {
                widths[column] = cell_len;
            }
        }
    }

    // Print the header row, then a divider, then the data rows.
    print_padded_row(&headers.map(|h| h.to_string()), &widths);
    print_divider(&widths);
    for row in &rows {
        print_padded_row(row, &widths);
    }
}

/// Print one row, each cell left-padded to its column width, space-separated.
fn print_padded_row(cells: &[String; 7], widths: &[usize; 7]) {
    let mut line = String::new();
    for column in 0..7 {
        if column > 0 {
            line.push_str("  "); // two spaces between columns
        }
        line.push_str(&pad_right(&cells[column], widths[column]));
    }
    // Trim trailing spaces from the last (COMMAND) column for tidiness.
    println!("{}", line.trim_end());
}

/// Print a dashed divider that lines up under the header.
fn print_divider(widths: &[usize; 7]) {
    let mut line = String::new();
    for column in 0..7 {
        if column > 0 {
            line.push_str("  ");
        }
        for _ in 0..widths[column] {
            line.push('-');
        }
    }
    println!("{}", line.trim_end());
}

/// Pad `text` on the right with spaces until it is `width` characters wide.
fn pad_right(text: &str, width: usize) -> String {
    let mut padded = String::from(text);
    let current = text.chars().count();
    for _ in current..width {
        padded.push(' ');
    }
    padded
}

/// Show a pid, or "-" when it is unknown, so empty cells are obvious.
fn format_pid(pid: i32) -> String {
    if pid == 0 {
        String::from("-")
    } else {
        pid.to_string()
    }
}

/// Replace an empty field with "-" so the table has no blank holes.
fn non_empty(text: &str) -> String {
    if text.is_empty() {
        String::from("-")
    } else {
        text.to_string()
    }
}

/// Print usage text.
fn print_help() {
    println!("portspy - see what is listening on your TCP ports\n");
    println!("usage:");
    println!("  portspy              list listening ports (default)");
    println!("  portspy print        list listening ports");
    println!("  portspy serve        web dashboard on 127.0.0.1:{DEFAULT_SERVE_PORT}");
    println!("  portspy serve <port> web dashboard on a chosen port");
    println!("  portspy help         show this help");
}
