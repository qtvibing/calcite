use std::{
    env,
    io::{self, IsTerminal, Read},
    process::ExitCode,
};

use calcite_core::{Diagnostic, calculate};

fn main() -> ExitCode {
    let expression = match read_expression() {
        Ok(expression) => expression,
        Err(error) => {
            eprintln!("calcite: {error}");
            return ExitCode::FAILURE;
        }
    };

    if expression.is_empty() {
        eprintln!("usage: calcite <expression>");
        return ExitCode::from(2);
    }

    match calculate(&expression) {
        Ok(result) => {
            println!("{}", result.output);
            ExitCode::SUCCESS
        }
        Err(diagnostic) => {
            print_diagnostic(&expression, &diagnostic);
            ExitCode::FAILURE
        }
    }
}

fn read_expression() -> io::Result<String> {
    let arguments: Vec<_> = env::args().skip(1).collect();
    if !arguments.is_empty() {
        return Ok(arguments.join(" "));
    }
    if io::stdin().is_terminal() {
        return Ok(String::new());
    }

    let mut source = String::new();
    io::stdin().read_to_string(&mut source)?;
    Ok(source.trim().to_owned())
}

fn print_diagnostic(source: &str, diagnostic: &Diagnostic) {
    eprintln!("error: {diagnostic}");
    eprintln!("  {source}");

    let span = diagnostic.span();
    let prefix_width = source[..span.start].chars().count();
    let marker_width = source[span.start..span.end].chars().count().max(1);
    eprintln!("  {}{}", " ".repeat(prefix_width), "^".repeat(marker_width));
}
