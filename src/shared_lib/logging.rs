#[macro_export]
macro_rules! log {
    ($first:expr) => {{
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        if !$first.is_empty() {
            stdout.write_all(b"\x1b[1;34m").unwrap();
            write!(&mut stdout, "{}", $first).unwrap();
            stdout.write_all(b"\x1b[0m").unwrap();
        }

        stdout.write_all(b"\n").unwrap();
        stdout.flush().unwrap();
    }};
    ($first:expr, $($arg:expr),* ) => {{
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        if !$first.is_empty() {
            stdout.write_all(b"\x1b[1;34m").unwrap();
            write!(&mut stdout, "{}:", $first).unwrap();
            stdout.write_all(b"\x1b[0m").unwrap();
        }

        $(
            write!(&mut stdout, " {}", $arg).unwrap();
        )*
            stdout.write_all(b"\n").unwrap();
        stdout.flush().unwrap();
    }};
}

#[macro_export]
macro_rules! die {
    ($first:expr) => {{
        use std::io::Write;
        let stderr = std::io::stderr();
        let mut stderr = stderr.lock();

        if !$first.is_empty() {
            stderr.write_all(b"\x1b[1;34m").unwrap();
            write!(&mut stderr, "{}", $first).unwrap();
            stderr.write_all(b"\x1b[0m").unwrap();
        }

        stderr.write_all(b"\n").unwrap();
        stderr.flush().unwrap();
        std::process::exit(pkg_clean());
    }};
    ($first:expr, $($arg:expr),* ) => {{
        use std::io::Write;
        let stderr = std::io::stderr();
        let mut stderr = stderr.lock();

        if !$first.is_empty() {
            stderr.write_all(b"\x1b[1;34m").unwrap();
            write!(&mut stderr, "{}:", $first).unwrap();
            stderr.write_all(b"\x1b[0m").unwrap();
        }

        $(
            write!(&mut stderr, " {}", $arg).unwrap();
        )*
            stderr.write_all(b"\n").unwrap();
        stderr.flush().unwrap();
        std::process::exit(pkg_clean());
    }};
}
