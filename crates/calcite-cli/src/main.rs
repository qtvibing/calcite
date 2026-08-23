mod rates;

use std::{
    env, fs,
    io::{self, IsTerminal, Read, Write},
    path::PathBuf,
    process::ExitCode,
};

use calcite_core::{
    CalculationContext, CalculationSession, ClockTime, Date, DateOrder, Diagnostic, DiagnosticKind,
    DocumentCalculation, NumberOutputMode, OffsetDateTime, UtcOffset,
    calculate_document_with_context,
};
use jiff::{Timestamp, tz::TimeZone};

const USAGE: &str = "Usage: calcite [OPTIONS] [EXPRESSION]\n\
\n\
Options:\n\
  -e, --eval <EXPRESSION>  Evaluate one expression\n\
  -f, --file <PATH>        Evaluate a Calcite document\n\
  -i, --repl               Start an interactive session\n\
      --refresh-rates      Refresh exchange rates before evaluation\n\
      --date-order <ORDER> Parse numeric dates as dmy, mdy, or ymd\n\
      --format <MODE>      auto, exact, fraction, mixed, decimal, significant, or scientific\n\
      --precision <N>      Digits used by decimal/scientific output\n\
      --approximate        Mark rounded or approximate numbers with ≈\n\
      --plain              Print only the final result\n\
      --json               Print structured JSON\n\
  -h, --help               Print help\n\
  -V, --version            Print version";

struct Options {
    action: Action,
    refresh_rates: bool,
    output: OutputMode,
    date_order: Option<DateOrder>,
    number_mode: NumberOutputMode,
    mark_approximate: bool,
}

#[derive(Clone, Copy)]
enum OutputMode {
    Text,
    Plain,
    Json,
}

#[derive(Clone, Copy)]
enum RequestedNumberMode {
    Auto,
    Exact,
    Fraction,
    Mixed,
    Decimal,
    Significant,
    Scientific,
}

impl RequestedNumberMode {
    const fn resolve(self, precision: u8) -> NumberOutputMode {
        match self {
            Self::Auto => NumberOutputMode::Auto,
            Self::Exact => NumberOutputMode::Exact,
            Self::Fraction => NumberOutputMode::Fraction,
            Self::Mixed => NumberOutputMode::Mixed,
            Self::Decimal => NumberOutputMode::Decimal { places: precision },
            Self::Significant => NumberOutputMode::Significant { digits: precision },
            Self::Scientific => NumberOutputMode::Scientific { digits: precision },
        }
    }
}

enum Action {
    Calculate(String),
    Help,
    Version,
    Repl,
}

#[tokio::main]
async fn main() -> ExitCode {
    let options = match read_options() {
        Ok(options) => options,
        Err(error) => {
            eprintln!("calcite: {error}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let expression = match options.action {
        Action::Help => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Action::Version => {
            println!("calcite {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Action::Repl => {
            return run_repl(
                options.refresh_rates,
                options.date_order,
                options.number_mode,
                options.mark_approximate,
                options.output,
            )
            .await;
        }
        Action::Calculate(expression) => expression,
    };

    if expression.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }

    match calculate_with_rates(
        &expression,
        options.refresh_rates,
        options.date_order,
        options.number_mode,
        options.mark_approximate,
    )
    .await
    {
        Ok(document) => {
            print_result(&document, options.output);
            ExitCode::SUCCESS
        }
        Err(diagnostic) => {
            if matches!(options.output, OutputMode::Json) {
                print_json_error(&diagnostic);
            } else {
                print_diagnostic(&expression, &diagnostic);
            }
            ExitCode::FAILURE
        }
    }
}

async fn run_repl(
    force_refresh: bool,
    date_order: Option<DateOrder>,
    number_mode: NumberOutputMode,
    mark_approximate: bool,
    output: OutputMode,
) -> ExitCode {
    let path = rates::cache_path();
    let cached = match rates::load_cache(&path) {
        Ok(cached) => cached,
        Err(error) => {
            eprintln!("warning: could not read exchange-rate cache: {error}");
            None
        }
    };
    let mut context = CalculationContext::default();
    context.temporal.date_order = date_order;
    context.output_format.number_mode = number_mode;
    context.output_format.mark_approximate = mark_approximate;
    if let Some(cached) = &cached
        && cached.fresh
    {
        context
            .monetary
            .exchange_rates
            .reload(cached.snapshot.clone());
    }
    let mut attempted_rates = false;
    if force_refresh {
        attempted_rates = true;
        if let Err(error) = refresh_rates(&mut context, &path).await {
            eprintln!("warning: {error}");
            if let Some(cached) = &cached {
                context
                    .monetary
                    .exchange_rates
                    .reload(cached.snapshot.clone());
            }
        }
    }

    let interactive = io::stdin().is_terminal();
    let mut session = CalculationSession::new();
    let mut history = Vec::new();
    loop {
        if interactive {
            print!("> ");
            if let Err(error) = io::stdout().flush() {
                eprintln!("calcite: {error}");
                return ExitCode::FAILURE;
            }
        }

        let mut source = String::new();
        match io::stdin().read_line(&mut source) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                eprintln!("calcite: {error}");
                return ExitCode::FAILURE;
            }
        }
        let source = source.trim();
        if source.is_empty() {
            continue;
        }
        match source {
            ":quit" | ":exit" => break,
            ":clear" => {
                session.clear();
                continue;
            }
            ":history" => {
                for (index, entry) in history.iter().enumerate() {
                    println!("{}  {entry}", index + 1);
                }
                continue;
            }
            _ => history.push(source.to_owned()),
        }

        let mut result = session.calculate(source, &context);
        if result
            .as_ref()
            .is_err_and(|error| error.kind() == DiagnosticKind::MissingTemporalContext)
        {
            install_current_datetime(&mut context);
            result = session.calculate(source, &context);
        }
        if !attempted_rates
            && result.as_ref().is_err_and(|error| {
                matches!(
                    error.kind(),
                    DiagnosticKind::MissingExchangeRate | DiagnosticKind::StaleExchangeRate
                )
            })
        {
            attempted_rates = true;
            if let Err(error) = refresh_rates(&mut context, &path).await {
                eprintln!("warning: {error}");
                if let Some(cached) = &cached {
                    context
                        .monetary
                        .exchange_rates
                        .reload(cached.snapshot.clone());
                }
            }
            result = session.calculate(source, &context);
        }

        match result {
            Ok(calculation) if matches!(output, OutputMode::Json) => println!(
                "{}",
                serde_json::json!({ "ok": true, "result": { "output": calculation.output } })
            ),
            Ok(calculation) => println!("{}", calculation.output),
            Err(error) if matches!(output, OutputMode::Json) => print_json_error(&error),
            Err(error) => print_diagnostic(source, &error),
        }
    }
    ExitCode::SUCCESS
}

async fn calculate_with_rates(
    expression: &str,
    force_refresh: bool,
    date_order: Option<DateOrder>,
    number_mode: NumberOutputMode,
    mark_approximate: bool,
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
    context.temporal.date_order = date_order;
    context.output_format.number_mode = number_mode;
    context.output_format.mark_approximate = mark_approximate;

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

    let mut result = calculate_with_local_time(expression, &mut context);
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
        result = calculate_with_local_time(expression, &mut context);
    }

    result
}

fn calculate_with_local_time(
    expression: &str,
    context: &mut CalculationContext,
) -> Result<DocumentCalculation, Diagnostic> {
    let result = calculate_document_with_context(expression, context);
    if context.temporal.current_datetime.is_none()
        && result
            .as_ref()
            .is_err_and(|diagnostic| diagnostic.kind() == DiagnosticKind::MissingTemporalContext)
    {
        install_current_datetime(context);
        calculate_document_with_context(expression, context)
    } else {
        result
    }
}

fn install_current_datetime(context: &mut CalculationContext) {
    let timezone = match TimeZone::try_system() {
        Ok(timezone) => timezone,
        Err(error) => {
            eprintln!("warning: could not determine the local timezone; using UTC: {error}");
            TimeZone::UTC
        }
    };
    let now = Timestamp::now().to_zoned(timezone);
    let date = Date::new(
        i32::from(now.year()),
        u8::try_from(now.month()).expect("Jiff months fit in u8"),
        u8::try_from(now.day()).expect("Jiff days fit in u8"),
    )
    .expect("the current date is in Calcite's supported range");
    let time = ClockTime::with_nanosecond(
        u8::try_from(now.hour()).expect("Jiff hours fit in u8"),
        u8::try_from(now.minute()).expect("Jiff minutes fit in u8"),
        u8::try_from(now.second()).expect("Jiff seconds fit in u8"),
        u32::try_from(now.subsec_nanosecond()).expect("Jiff nanoseconds fit in u32"),
    )
    .expect("the current time is valid");
    let offset = UtcOffset::from_seconds(now.offset().seconds())
        .expect("IANA time-zone offsets fit in Calcite's supported range");
    context.temporal.current_datetime = Some(OffsetDateTime::new(date, time, offset));
    context.temporal.default_offset = offset;
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

fn read_options() -> Result<Options, String> {
    let mut refresh_rates = false;
    let mut output = OutputMode::Text;
    let mut date_order = None;
    let mut requested_number_mode = RequestedNumberMode::Auto;
    let mut precision = 10;
    let mut mark_approximate = false;
    let mut repl = false;
    let mut expression = None;
    let mut file = None;
    let mut positional = Vec::new();
    let mut arguments = env::args().skip(1);

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--refresh-rates" => refresh_rates = true,
            "-h" | "--help" => {
                return Ok(Options {
                    action: Action::Help,
                    refresh_rates,
                    output,
                    date_order,
                    number_mode: NumberOutputMode::Auto,
                    mark_approximate: false,
                });
            }
            "-V" | "--version" => {
                return Ok(Options {
                    action: Action::Version,
                    refresh_rates,
                    output,
                    date_order,
                    number_mode: NumberOutputMode::Auto,
                    mark_approximate: false,
                });
            }
            "--plain" => output = OutputMode::Plain,
            "--json" => output = OutputMode::Json,
            "--format" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--format requires an output mode".to_owned())?;
                requested_number_mode = match value.to_ascii_lowercase().as_str() {
                    "auto" => RequestedNumberMode::Auto,
                    "exact" => RequestedNumberMode::Exact,
                    "fraction" => RequestedNumberMode::Fraction,
                    "mixed" => RequestedNumberMode::Mixed,
                    "decimal" => RequestedNumberMode::Decimal,
                    "significant" => RequestedNumberMode::Significant,
                    "scientific" => RequestedNumberMode::Scientific,
                    _ => {
                        return Err(
                            "--format requires auto, exact, fraction, mixed, decimal, significant, or scientific"
                                .into(),
                        );
                    }
                };
            }
            "--precision" => {
                let value = arguments.next().ok_or_else(|| {
                    "--precision requires an integer from 1 through 100".to_owned()
                })?;
                precision = value
                    .parse::<u8>()
                    .ok()
                    .filter(|value| (1..=100).contains(value))
                    .ok_or_else(|| {
                        "--precision requires an integer from 1 through 100".to_owned()
                    })?;
            }
            "--approximate" => mark_approximate = true,
            "--date-order" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--date-order requires dmy, mdy, or ymd".to_owned())?;
                date_order = Some(match value.to_ascii_lowercase().as_str() {
                    "dmy" => DateOrder::DayMonthYear,
                    "mdy" => DateOrder::MonthDayYear,
                    "ymd" => DateOrder::YearMonthDay,
                    _ => return Err("--date-order requires dmy, mdy, or ymd".into()),
                });
            }
            "-i" | "--repl" => {
                if expression.is_some() || file.is_some() || !positional.is_empty() {
                    return Err("only one input source may be specified".into());
                }
                repl = true;
            }
            "-e" | "--eval" => {
                if repl || expression.is_some() || file.is_some() || !positional.is_empty() {
                    return Err("only one input source may be specified".into());
                }
                expression = Some(
                    arguments
                        .next()
                        .ok_or_else(|| format!("{argument} requires an expression"))?,
                );
            }
            "-f" | "--file" => {
                if repl || expression.is_some() || file.is_some() || !positional.is_empty() {
                    return Err("only one input source may be specified".into());
                }
                file = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| format!("{argument} requires a path"))?,
                ));
            }
            "--" => {
                positional.extend(arguments);
                break;
            }
            _ if argument.starts_with("--") => {
                return Err(format!("unknown option: {argument}"));
            }
            _ => positional.push(argument),
        }
    }

    if (repl || expression.is_some() || file.is_some()) && !positional.is_empty() {
        return Err("only one input source may be specified".into());
    }
    if repl {
        return Ok(Options {
            action: Action::Repl,
            refresh_rates,
            output,
            date_order,
            number_mode: requested_number_mode.resolve(precision),
            mark_approximate,
        });
    }
    let source = if let Some(expression) = expression {
        expression
    } else if let Some(path) = file {
        fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?
    } else if !positional.is_empty() {
        positional.join(" ")
    } else if io::stdin().is_terminal() {
        return Ok(Options {
            action: Action::Repl,
            refresh_rates,
            output,
            date_order,
            number_mode: requested_number_mode.resolve(precision),
            mark_approximate,
        });
    } else {
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|error| error.to_string())?;
        source
    };

    Ok(Options {
        action: Action::Calculate(source.trim().to_owned()),
        refresh_rates,
        output,
        date_order,
        number_mode: requested_number_mode.resolve(precision),
        mark_approximate,
    })
}

fn print_result(document: &DocumentCalculation, output: OutputMode) {
    match output {
        OutputMode::Text => {
            for line in &document.lines {
                println!("{}", line.output);
            }
        }
        OutputMode::Plain => {
            if let Some(line) = document.lines.last() {
                println!("{}", line.output);
            }
        }
        OutputMode::Json => {
            let results = document
                .lines
                .iter()
                .map(|line| {
                    serde_json::json!({
                        "name": line.name,
                        "output": line.output,
                        "span": { "start": line.span.start, "end": line.span.end },
                    })
                })
                .collect::<Vec<_>>();
            println!("{}", serde_json::json!({ "ok": true, "results": results }));
        }
    }
}

fn print_json_error(diagnostic: &Diagnostic) {
    println!(
        "{}",
        serde_json::json!({
            "ok": false,
            "error": {
                "kind": format!("{:?}", diagnostic.kind()),
                "message": diagnostic.to_string(),
                "span": {
                    "start": diagnostic.span().start,
                    "end": diagnostic.span().end,
                },
            },
        })
    );
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
