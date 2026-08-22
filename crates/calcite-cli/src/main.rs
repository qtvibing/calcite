mod rates;

use std::{
    env,
    io::{self, IsTerminal, Read},
    process::ExitCode,
};

use calcite_core::{
    CalculationContext, Diagnostic, DiagnosticKind, DocumentCalculation,
    calculate_document_with_context,
};

#[tokio::main]
async fn main() -> ExitCode {
    let (expression, refresh_rates) = match read_options() {
        Ok(options) => options,
        Err(error) => {
            eprintln!("calcite: {error}");
            return ExitCode::FAILURE;
        }
    };

    if expression.is_empty() {
        eprintln!("usage: calcite [--refresh-rates] <expression>");
        return ExitCode::from(2);
    }

    match calculate_with_rates(&expression, refresh_rates).await {
        Ok(document) => {
            for line in document.lines {
                println!("{}", line.output);
            }
            ExitCode::SUCCESS
        }
        Err(diagnostic) => {
            print_diagnostic(&expression, &diagnostic);
            ExitCode::FAILURE
        }
    }
}

async fn calculate_with_rates(
    expression: &str,
    force_refresh: bool,
) -> Result<DocumentCalculation, Diagnostic> {
    let path = rates::cache_path();
    let cached = match rates::load_cache(&path) {
        Ok(cached) => cached,
        Err(error) => {
            eprintln!("warning: could not read exchange-rate cache: {error}");
            None
        }
    };
    let mut context = CalculationContext::default();

    if force_refresh {
        if let Err(error) = refresh_rates(&mut context, &path).await {
            eprintln!("warning: {error}");
            if let Some(cached) = &cached {
                context
                    .monetary
                    .exchange_rates
                    .reload(cached.snapshot.clone());
            }
        }
    } else if let Some(cached) = &cached
        && cached.fresh
    {
        context
            .monetary
            .exchange_rates
            .reload(cached.snapshot.clone());
    }

    let mut result = calculate_document_with_context(expression, &context);
    let needs_rates = result.as_ref().is_err_and(|diagnostic| {
        matches!(
            diagnostic.kind(),
            DiagnosticKind::MissingExchangeRate | DiagnosticKind::StaleExchangeRate
        )
    });
    if needs_rates && !force_refresh {
        if let Err(error) = refresh_rates(&mut context, &path).await {
            eprintln!("warning: {error}");
            if let Some(cached) = &cached {
                context
                    .monetary
                    .exchange_rates
                    .reload(cached.snapshot.clone());
            }
        }
        result = calculate_document_with_context(expression, &context);
    }

    result
}

async fn refresh_rates(
    context: &mut CalculationContext,
    path: &std::path::Path,
) -> Result<(), calcite_rates::ProviderError> {
    let snapshot = rates::fetch_rates().await?;
    if let Err(error) = rates::save_cache(path, &snapshot) {
        eprintln!("warning: could not write exchange-rate cache: {error}");
    }
    context.monetary.exchange_rates.reload(snapshot);
    Ok(())
}

fn read_options() -> io::Result<(String, bool)> {
    let mut refresh_rates = false;
    let arguments = env::args()
        .skip(1)
        .filter(|argument| {
            if argument == "--refresh-rates" {
                refresh_rates = true;
                false
            } else {
                true
            }
        })
        .collect::<Vec<_>>();
    if !arguments.is_empty() {
        return Ok((arguments.join(" "), refresh_rates));
    }
    if io::stdin().is_terminal() {
        return Ok((String::new(), refresh_rates));
    }

    let mut source = String::new();
    io::stdin().read_to_string(&mut source)?;
    Ok((source.trim().to_owned(), refresh_rates))
}

fn print_diagnostic(source: &str, diagnostic: &Diagnostic) {
    eprintln!("error: {diagnostic}");

    let span = diagnostic.span();
    let line_start = source[..span.start]
        .rfind('\n')
        .map_or(0, |position| position + 1);
    let line_end = source[span.start..]
        .find('\n')
        .map_or(source.len(), |position| span.start + position);
    let line = &source[line_start..line_end];
    eprintln!("  {line}");

    let marker_end = span.end.min(line_end);
    let prefix_width = source[line_start..span.start].chars().count();
    let marker_width = source[span.start..marker_end].chars().count().max(1);
    eprintln!("  {}{}", " ".repeat(prefix_width), "^".repeat(marker_width));
}
