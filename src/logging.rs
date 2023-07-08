
// macros
#[macro_export]
macro_rules! log {
    ($first:expr $(, $arg:expr)*) => {
        {
            use std::io::Write;
            use termcolor::{ColorSpec, ColorChoice, StandardStream, WriteColor};

            let mut stdout: StandardStream = StandardStream::stdout(ColorChoice::Auto);

            stdout.set_color(ColorSpec::new().set_fg(Some(termcolor::Color::Yellow)).set_bold(true))
                .unwrap_or_else(|_| panic!("Failed to set color"));
            write!(&mut stdout, "-> ").unwrap();
            stdout.set_color(ColorSpec::new().set_fg(Some(termcolor::Color::Blue)).set_bold(true))
                .unwrap_or_else(|_| panic!("Failed to set color"));
            write!(&mut stdout, "{} ", $first).unwrap();
            stdout.reset().unwrap_or_else(|_| panic!("Failed to set color"));
            writeln!(&mut stdout, $($arg),*).unwrap();
        }
    };
}

#[macro_export]
macro_rules! die {
    ($first:expr $(, $arg:expr)*) => {
        {
            use std::io::Write;
            use termcolor::{ColorSpec, ColorChoice, StandardStream, WriteColor};
            use std::process::exit;
            use super::pkg_clean;

            let mut stdout: StandardStream = StandardStream::stderr(ColorChoice::Auto);

            stdout.set_color(ColorSpec::new().set_fg(Some(termcolor::Color::Yellow)).set_bold(true))
                .unwrap_or_else(|_| panic!("Failed to set color"));
            write!(&mut stdout, "ERROR ").unwrap();
            stdout.set_color(ColorSpec::new().set_fg(Some(termcolor::Color::Blue)).set_bold(true))
                .unwrap_or_else(|_| panic!("Failed to set color"));
            write!(&mut stdout, "{} ", $first).unwrap();
            stdout.reset().unwrap_or_else(|_| panic!("Failed to set color"));
            writeln!(&mut stdout, $($arg),*).unwrap();
            // exit
            exit(pkg_clean(1));
        }
    };
}
