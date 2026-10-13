mod config;
mod history;
mod rates;

use std::{
    env, fs,
    io::{self, IsTerminal, Read},
    path::PathBuf,
    process::ExitCode,
};

use calcite_core::{
    CalculationContext, CalculationSession, ClockTime, Currency, Date, DateOrder, Diagnostic,
    DiagnosticKind, DocumentCalculation, EntropySource, Number, NumberFormat, NumberOutputMode,
    OffsetDateTime, UtcOffset, Value, calculate_document_with_context_and_entropy,
};
use jiff::{Timestamp, tz::TimeZone};
use rustyline::{
    Config as ReadlineConfig, DefaultEditor,
    error::ReadlineError,
    history::{History as _, SearchDirection},
};

const USAGE: &str = "Usage: calcite [OPTIONS] [EXPRESSION]\n\
\n\
Options:\n\
  -e, --eval <EXPRESSION>  Evaluate one expression\n\
  -f, --file <PATH>        Evaluate a Calcite document\n\
  -i, --repl               Start an interactive session\n\
      --refresh-rates      Refresh exchange rates before evaluation\n\
      --config <PATH>      Load configuration from a JSON file\n\
      --locale <LOCALE>    Use en-US, en-GB, de-DE, or fr-FR number syntax\n\
      --currency <CODE>    Set the default ISO 4217 currency\n\
      --timezone <ZONE>    Use an IANA zone for contextual dates and times\n\
      --rate-provider <P>  Use fallback, frankfurter, or ecb\n\
      --no-history         Disable persistent interactive history\n\
      --date-order <ORDER> Parse numeric dates as dmy, mdy, or ymd\n\
      --format <MODE>      auto, plain, exact, fraction, mixed, decimal, significant, or scientific\n\
      --precision <N>      Digits used by decimal/scientific output\n\
      --approximate        Mark rounded or approximate numbers with ≈\n\
      --plain              Print only the final result\n\
      --json               Print structured JSON\n\
  -h, --help               Print help\n\
  -V, --version            Print version";

#[derive(Default)]
struct OsEntropy {
    values: Vec<u64>,
    position: usize,
}

impl OsEntropy {
    fn rewind(&mut self) {
        self.position = 0;
    }
}

impl EntropySource for OsEntropy {
    fn next_u64(&mut self) -> Option<u64> {
        if let Some(value) = self.values.get(self.position).copied() {
            self.position += 1;
            return Some(value);
        }
        let mut bytes = [0_u8; size_of::<u64>()];
        getrandom::fill(&mut bytes).ok()?;
        let value = u64::from_ne_bytes(bytes);
        self.values.push(value);
        self.position += 1;
        Some(value)
    }
}

struct Options {
    action: Action,
    refresh_rates: bool,
    output: OutputMode,
    date_order: Option<DateOrder>,
    number_format: NumberFormat,
    number_mode: NumberOutputMode,
    mark_approximate: bool,
    default_currency: Currency,
    custom_units: calcite_core::CustomUnitRegistry,
    timezone: Option<String>,
    rate_provider: rates::ProviderChoice,
    history: bool,
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
    Plain,
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
            Self::Plain => NumberOutputMode::Plain,
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
    let expression = match &options.action {
        Action::Help => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Action::Version => {
            println!("calcite {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Action::Repl => return run_repl(&options).await,
        Action::Calculate(expression) => expression,
    };

    if expression.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }

    match calculate_with_rates(expression, &options).await {
        Ok(document) => {
            print_result(&document, options.output);
            ExitCode::SUCCESS
        }
        Err(diagnostic) => {
            if matches!(options.output, OutputMode::Json) {
                print_json_error(&diagnostic);
            } else {
                print_diagnostic(expression, &diagnostic);
            }
            ExitCode::FAILURE
        }
    }
}

async fn run_repl(options: &Options) -> ExitCode {
    let force_refresh = options.refresh_rates;
    let timezone = options.timezone.as_deref();
    let rate_provider = options.rate_provider;
    let output = options.output;
    let path = rates::cache_path();
    let cached = match rates::load_cache(&path) {
        Ok(cached) => cached,
        Err(error) => {
            eprintln!("warning: could not read exchange-rate cache: {error}");
            None
        }
    };
    let mut context = CalculationContext::default();
    context.temporal.date_order = options.date_order;
    context.number_format = options.number_format;
    context.monetary.default_currency = options.default_currency;
    context.custom_units = options.custom_units.clone();
    context.output_format.number_mode = options.number_mode;
    context.output_format.mark_approximate = options.mark_approximate;
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
        if let Err(error) = refresh_rates(&mut context, &path, rate_provider).await {
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
    let history_path = options.history.then(history::default_path);
    let mut editor = if interactive {
        match ReadlineConfig::builder()
            .max_history_size(1_000)
            .and_then(|builder| builder.history_ignore_dups(true))
            .and_then(|builder| DefaultEditor::with_config(builder.build()))
        {
            Ok(mut editor) => {
                if let Some(path) = &history_path
                    && path.exists()
                    && let Err(error) = editor.load_history(path)
                {
                    eprintln!("warning: could not read calculation history: {error}");
                }
                Some(editor)
            }
            Err(error) => {
                eprintln!("warning: interactive line editing is unavailable: {error}");
                None
            }
        }
    } else {
        None
    };
    let mut transient_history = editor.as_ref().map(readline_history).unwrap_or_default();
    loop {
        let source = if let Some(editor) = &mut editor {
            match editor.readline("> ") {
                Ok(source) => source,
                Err(ReadlineError::Interrupted) => continue,
                Err(ReadlineError::Eof) => break,
                Err(error) => {
                    eprintln!("calcite: {error}");
                    return ExitCode::FAILURE;
                }
            }
        } else {
            let mut source = String::new();
            match io::stdin().read_line(&mut source) {
                Ok(0) => break,
                Ok(_) => source,
                Err(error) => {
                    eprintln!("calcite: {error}");
                    return ExitCode::FAILURE;
                }
            }
        };
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
            ":clear-history" => {
                transient_history.clear();
                if let Some(editor) = &mut editor {
                    if let Err(error) = editor.clear_history() {
                        eprintln!("warning: could not clear calculation history: {error}");
                    }
                    if let Some(path) = &history_path {
                        let _ = fs::remove_file(path);
                    }
                }
                continue;
            }
            ":history" => {
                for (index, entry) in transient_history.iter().enumerate() {
                    println!("{}  {entry}", index + 1);
                }
                continue;
            }
            _ => {}
        }
        let recalled = resolve_transient_history(&transient_history, source).map(str::to_owned);
        let source = recalled.as_deref().unwrap_or(source);
        if let Some(recalled) = &recalled
            && interactive
        {
            println!("{recalled}");
        }
        if transient_history.last().is_none_or(|entry| entry != source) {
            transient_history.push(source.to_owned());
        }
        if let Some(editor) = &mut editor {
            if let Err(error) = editor.add_history_entry(source) {
                eprintln!("warning: could not update calculation history: {error}");
            } else if let Some(path) = &history_path
                && let Err(error) = history::ensure_parent(path)
                    .map_err(|error| error.to_string())
                    .and_then(|()| editor.save_history(path).map_err(|error| error.to_string()))
            {
                eprintln!("warning: could not write calculation history: {error}");
            }
        }

        let mut entropy = OsEntropy::default();
        let mut result = session.calculate_with_entropy(source, &context, &mut entropy);
        if result
            .as_ref()
            .is_err_and(|error| error.kind() == DiagnosticKind::MissingTemporalContext)
        {
            install_current_datetime(&mut context, timezone);
            entropy.rewind();
            result = session.calculate_with_entropy(source, &context, &mut entropy);
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
            if let Err(error) = refresh_rates(&mut context, &path, rate_provider).await {
                eprintln!("warning: {error}");
                if let Some(cached) = &cached {
                    context
                        .monetary
                        .exchange_rates
                        .reload(cached.snapshot.clone());
                }
            }
            entropy.rewind();
            result = session.calculate_with_entropy(source, &context, &mut entropy);
        }

        match result {
            Ok(calculation) if matches!(output, OutputMode::Json) => println!(
                "{}",
                serde_json::json!({
                    "ok": true,
                    "result": {
                        "output": calculation.output,
                        "value": typed_json_value(&calculation.value),
                    }
                })
            ),
            Ok(calculation) => println!("{}", calculation.output),
            Err(error) if matches!(output, OutputMode::Json) => print_json_error(&error),
            Err(error) => print_diagnostic(source, &error),
        }
    }
    ExitCode::SUCCESS
}

fn readline_history(editor: &DefaultEditor) -> Vec<String> {
    (0..editor.history().len())
        .filter_map(|index| {
            editor
                .history()
                .get(index, SearchDirection::Forward)
                .ok()
                .flatten()
                .map(|entry| entry.entry.into_owned())
        })
        .collect()
}

fn resolve_transient_history<'a>(history: &'a [String], source: &str) -> Option<&'a str> {
    if source == "!!" {
        return history.last().map(String::as_str);
    }
    source
        .strip_prefix('!')?
        .parse::<usize>()
        .ok()
        .and_then(|index| index.checked_sub(1))
        .and_then(|index| history.get(index))
        .map(String::as_str)
}

async fn calculate_with_rates(
    expression: &str,
    options: &Options,
) -> Result<DocumentCalculation, Diagnostic> {
    let force_refresh = options.refresh_rates;
    let timezone = options.timezone.as_deref();
    let rate_provider = options.rate_provider;
    let path = rates::cache_path();
    let cached = match rates::load_cache(&path) {
        Ok(cached) => cached,
        Err(error) => {
            eprintln!("warning: could not read exchange-rate cache: {error}");
            None
        }
    };
    let mut context = CalculationContext::default();
    context.temporal.date_order = options.date_order;
    context.number_format = options.number_format;
    context.monetary.default_currency = options.default_currency;
    context.custom_units = options.custom_units.clone();
    context.output_format.number_mode = options.number_mode;
    context.output_format.mark_approximate = options.mark_approximate;

    if force_refresh {
        if let Err(error) = refresh_rates(&mut context, &path, rate_provider).await {
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

    let mut entropy = OsEntropy::default();
    let mut result = calculate_with_local_time(expression, &mut context, timezone, &mut entropy);
    let needs_rates = result.as_ref().is_err_and(|diagnostic| {
        matches!(
            diagnostic.kind(),
            DiagnosticKind::MissingExchangeRate | DiagnosticKind::StaleExchangeRate
        )
    });
    if needs_rates && !force_refresh {
        if let Err(error) = refresh_rates(&mut context, &path, rate_provider).await {
            eprintln!("warning: {error}");
            if let Some(cached) = &cached {
                context
                    .monetary
                    .exchange_rates
                    .reload(cached.snapshot.clone());
            }
        }
        entropy.rewind();
        result = calculate_with_local_time(expression, &mut context, timezone, &mut entropy);
    }

    result
}

fn calculate_with_local_time(
    expression: &str,
    context: &mut CalculationContext,
    timezone: Option<&str>,
    entropy: &mut OsEntropy,
) -> Result<DocumentCalculation, Diagnostic> {
    let result = calculate_document_with_context_and_entropy(expression, context, entropy);
    if context.temporal.current_datetime.is_none()
        && result
            .as_ref()
            .is_err_and(|diagnostic| diagnostic.kind() == DiagnosticKind::MissingTemporalContext)
    {
        install_current_datetime(context, timezone);
        entropy.rewind();
        calculate_document_with_context_and_entropy(expression, context, entropy)
    } else {
        result
    }
}

fn install_current_datetime(context: &mut CalculationContext, configured_timezone: Option<&str>) {
    let timezone = match configured_timezone {
        Some(name) => match TimeZone::get(name) {
            Ok(timezone) => timezone,
            Err(error) => {
                eprintln!(
                    "warning: could not load configured timezone '{name}'; using UTC: {error}"
                );
                TimeZone::UTC
            }
        },
        None => match TimeZone::try_system() {
            Ok(timezone) => timezone,
            Err(error) => {
                eprintln!("warning: could not determine the local timezone; using UTC: {error}");
                TimeZone::UTC
            }
        },
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
    provider: rates::ProviderChoice,
) -> Result<(), calcite_rates::ProviderError> {
    let snapshot = rates::fetch_rates(provider).await?;
    if let Err(error) = rates::save_cache(path, &snapshot) {
        eprintln!("warning: could not write exchange-rate cache: {error}");
    }
    context.monetary.exchange_rates.reload(snapshot);
    Ok(())
}

fn read_options() -> Result<Options, String> {
    let raw_arguments = env::args().skip(1).collect::<Vec<_>>();
    let explicit_config = raw_arguments
        .iter()
        .position(|argument| argument == "--config")
        .map(|position| {
            raw_arguments
                .get(position + 1)
                .map(PathBuf::from)
                .ok_or_else(|| "--config requires a path".to_owned())
        })
        .transpose()?;
    let defaults = config::load(explicit_config)?;
    let mut refresh_rates = false;
    let mut output = OutputMode::Text;
    let mut date_order = defaults.date_order;
    let mut number_format = defaults.number_format;
    let mut requested_number_mode = defaults.number_mode;
    let mut precision = defaults.precision;
    let mut mark_approximate = defaults.approximate;
    let mut default_currency = defaults.default_currency;
    let custom_units = defaults.custom_units;
    let mut timezone = defaults.timezone;
    let mut rate_provider = defaults.rate_provider;
    let mut history = defaults.history;
    let mut repl = false;
    let mut expression = None;
    let mut file = None;
    let mut positional = Vec::new();
    let mut arguments = raw_arguments.into_iter();

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--refresh-rates" => refresh_rates = true,
            "--config" => {
                arguments
                    .next()
                    .ok_or_else(|| "--config requires a path".to_owned())?;
            }
            "--locale" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--locale requires a locale".to_owned())?;
                number_format = config::parse_locale(&value)?;
            }
            "--currency" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--currency requires an ISO 4217 code".to_owned())?;
                default_currency = Currency::from_code(&value)
                    .ok_or_else(|| "--currency requires an ISO 4217 code".to_owned())?;
            }
            "--timezone" => {
                timezone = Some(
                    arguments
                        .next()
                        .ok_or_else(|| "--timezone requires an IANA timezone name".to_owned())?,
                );
            }
            "--rate-provider" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--rate-provider requires a provider".to_owned())?;
                rate_provider = config::parse_provider(&value)?;
            }
            "--no-history" => history = false,
            "-h" | "--help" => {
                return Ok(Options {
                    action: Action::Help,
                    refresh_rates,
                    output,
                    date_order,
                    number_format,
                    number_mode: requested_number_mode.resolve(precision),
                    mark_approximate,
                    default_currency,
                    custom_units,
                    timezone,
                    rate_provider,
                    history,
                });
            }
            "-V" | "--version" => {
                return Ok(Options {
                    action: Action::Version,
                    refresh_rates,
                    output,
                    date_order,
                    number_format,
                    number_mode: requested_number_mode.resolve(precision),
                    mark_approximate,
                    default_currency,
                    custom_units,
                    timezone,
                    rate_provider,
                    history,
                });
            }
            "--plain" => output = OutputMode::Plain,
            "--json" => output = OutputMode::Json,
            "--format" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--format requires an output mode".to_owned())?;
                requested_number_mode = config::parse_format(&value)?;
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
                date_order = Some(config::parse_date_order(&value)?);
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

    if let Some(name) = &timezone {
        TimeZone::get(name).map_err(|error| format!("invalid timezone '{name}': {error}"))?;
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
            number_format,
            number_mode: requested_number_mode.resolve(precision),
            mark_approximate,
            default_currency,
            custom_units,
            timezone,
            rate_provider,
            history,
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
            number_format,
            number_mode: requested_number_mode.resolve(precision),
            mark_approximate,
            default_currency,
            custom_units,
            timezone,
            rate_provider,
            history,
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
        number_format,
        number_mode: requested_number_mode.resolve(precision),
        mark_approximate,
        default_currency,
        custom_units,
        timezone,
        rate_provider,
        history,
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
                        "value": typed_json_value(&line.value),
                        "span": { "start": line.span.start, "end": line.span.end },
                    })
                })
                .collect::<Vec<_>>();
            println!("{}", serde_json::json!({ "ok": true, "results": results }));
        }
    }
}

fn typed_json_value(value: &Value) -> serde_json::Value {
    match value {
        Value::Number(value) => number_json("number", value),
        Value::String(value) => serde_json::json!({ "type": "string", "value": value }),
        Value::RadixInteger(value) => serde_json::json!({
            "type": "radix_integer",
            "value": value.value().exact_fraction(),
            "radix": value.radix().value(),
        }),
        Value::RomanInteger(value) => serde_json::json!({
            "type": "roman_integer",
            "value": value.value().exact_fraction(),
            "roman": value.to_string(),
        }),
        Value::Complex(value) => serde_json::json!({
            "type": "complex",
            "real": number_json("number", value.real()),
            "imaginary": number_json("number", value.imaginary()),
        }),
        Value::Function(value) => serde_json::json!({
            "type": "function",
            "parameters": value.parameters(),
        }),
        Value::Percentage(value) => serde_json::json!({
            "type": "percentage",
            "ratio": canonical_number(value.ratio()),
        }),
        Value::Fraction(value) => serde_json::json!({
            "type": "fraction",
            "value": canonical_number(value.number()),
        }),
        Value::Unit(value) => serde_json::json!({
            "type": "unit",
            "magnitude": canonical_number(value.magnitude()),
            "unit": value.compound_unit().to_string(),
        }),
        Value::CustomUnit(value) => serde_json::json!({
            "type": "custom_unit",
            "magnitude": canonical_number(value.magnitude()),
            "unit": value.definition().name(),
            "scale": canonical_number(value.definition().scale()),
            "defined_as": value.definition().unit().to_string(),
        }),
        Value::Money(value) => serde_json::json!({
            "type": "money",
            "amount": canonical_number(value.amount()),
            "currency": value.currency().to_string(),
        }),
        Value::MoneyRate(value) => serde_json::json!({
            "type": "money_rate",
            "amount": canonical_number(value.amount()),
            "currency": value.currency().to_string(),
            "denominator": value.denominator().to_string(),
        }),
        Value::Rate(value) => serde_json::json!({
            "type": "rate",
            "magnitude": canonical_number(value.magnitude()),
            "unit": value.compound_unit().to_string(),
        }),
        Value::Dice(value) => serde_json::json!({
            "type": "dice_distribution",
            "count": value.count(),
            "sides": value.sides(),
            "modifier": value.modifier(),
            "minimum": value.minimum(),
            "maximum": value.maximum(),
        }),
        Value::Coin(value) => textual_json("coin_side", value),
        Value::Date(value) => textual_json("date", value),
        Value::Clock(value) => textual_json("clock_time", value),
        Value::OffsetClock(value) => textual_json("offset_clock_time", value),
        Value::ZonedClock(value) => textual_json("zoned_clock_time", value),
        Value::OffsetDateTime(value) => textual_json("offset_date_time", value),
        Value::ZonedDateTime(value) => textual_json("zoned_date_time", value),
        Value::TimeSpan(value) => textual_json("time_span", value),
        Value::CalendarDuration(value) => textual_json("calendar_duration", value),
        Value::CalendarSpan(value) => textual_json("calendar_span", value),
        Value::WorkdayDuration(value) => textual_json("workday_duration", value),
        Value::Weekday(value) => textual_json("weekday", value),
        Value::Boolean(value) => serde_json::json!({ "type": "boolean", "value": value }),
        _ => serde_json::json!({ "type": "unknown" }),
    }
}

fn number_json(kind: &str, value: &Number) -> serde_json::Value {
    serde_json::json!({
        "type": kind,
        "value": canonical_number(value),
        "exact": value.is_exact(),
    })
}

fn canonical_number(value: &Number) -> String {
    value.exact_fraction().unwrap_or_else(|| value.to_string())
}

fn textual_json(kind: &str, value: &impl ToString) -> serde_json::Value {
    serde_json::json!({ "type": kind, "value": value.to_string() })
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
