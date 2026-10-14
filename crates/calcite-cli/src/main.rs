mod config;
mod history;
mod rates;

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{self, IsTerminal, Read},
    path::PathBuf,
    process::ExitCode,
};

use calcite_core::{
    CalculationContext, CalculationSession, CalendarDurationUnit, ClockTime, Currency, Date,
    DateOrder, Diagnostic, DiagnosticKind, DocumentCalculation, EntropySource, Function, Number,
    NumberFormat, NumberOutputMode, OffsetDateTime, TimeZoneCode, TimeZoneDefinition,
    TimeZoneSnapshot, TimeZoneTransition, UtcOffset, Value,
    calculate_document_with_context_and_entropy, surface_variants,
};
use jiff::{Timestamp, civil, tz::TimeZone};
use rustyline::{
    Config as ReadlineConfig, Editor,
    completion::{Completer, Pair},
    error::ReadlineError,
    highlight::Highlighter,
    hint::Hinter,
    history::{FileHistory, History as _, SearchDirection},
    validate::Validator,
};

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
        getrandom::getrandom(&mut bytes).ok()?;
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
    literal_zone_codes: bool,
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

use clap::{CommandFactory as _, Parser as _};

#[derive(clap::Parser)]
#[command(
    name = "calcite",
    version,
    about = "A calculator for expressions written the way people write them"
)]
struct Cli {
    /// Evaluate one expression
    #[arg(short = 'e', long)]
    eval: Option<String>,

    /// Evaluate a Calcite document from a file
    #[arg(short = 'f', long)]
    file: Option<PathBuf>,

    /// Start an interactive session
    #[arg(short = 'i', long)]
    repl: bool,

    /// Refresh exchange rates before evaluation
    #[arg(long)]
    refresh_rates: bool,

    /// Load configuration from a JSON file
    #[arg(long)]
    config: Option<PathBuf>,

    /// Use en-US, en-GB, de-DE, or fr-FR number syntax
    #[arg(long)]
    locale: Option<String>,

    /// Set the default ISO 4217 currency
    #[arg(long)]
    currency: Option<String>,

    /// Use an IANA zone for contextual dates and times
    #[arg(long)]
    timezone: Option<String>,

    /// Interpret codes like CET as their fixed offset instead of following
    /// the daylight-saving rules of their region
    #[arg(long)]
    literal_zone_codes: bool,

    /// Use fallback, frankfurter, or ecb
    #[arg(long = "rate-provider")]
    rate_provider: Option<String>,

    /// Disable persistent interactive history
    #[arg(long)]
    no_history: bool,

    /// Parse numeric dates as dmy, mdy, or ymd
    #[arg(long = "date-order")]
    date_order: Option<String>,

    /// auto, plain, exact, fraction, mixed, decimal, significant, or scientific
    #[arg(long)]
    format: Option<String>,

    /// Digits used by decimal/scientific output (1 through 100)
    #[arg(long)]
    precision: Option<String>,

    /// Mark rounded or approximate numbers with ≈
    #[arg(long)]
    approximate: bool,

    /// Print only the final result
    #[arg(long)]
    plain: bool,

    /// Print structured JSON
    #[arg(long)]
    json: bool,

    /// Expression to evaluate. Multiple words are joined.
    #[arg(allow_hyphen_values = true)]
    expression: Vec<String>,
}

enum Action {
    Calculate(String),
    Repl,
}

#[tokio::main]
async fn main() -> ExitCode {
    #[cfg(unix)]
    // let a closed pipe end the process by signal instead of panicking on write
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    let options = match read_options() {
        Ok(options) => options,
        Err(error) => {
            eprintln!("calcite: {error}");
            return ExitCode::from(2);
        }
    };
    let expression = match options.action {
        Action::Repl => return run_repl(&options).await,
        Action::Calculate(ref expression) => expression.clone(),
    };

    if expression.is_empty() {
        eprintln!("{}", Cli::command().render_usage());
        return ExitCode::from(2);
    }

    match calculate_surface_variants(&expression, &options).await {
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

/// Tries the raw input, then normalized conversational variants.
///
/// Normalized variants rescue only input that the language rejects. The
/// reported diagnostic always comes from the raw input. Error spans then
/// match what the user typed.
async fn calculate_surface_variants(
    expression: &str,
    options: &Options,
) -> Result<DocumentCalculation, Diagnostic> {
    let mut first = None;
    for variant in surface_variants(expression) {
        match calculate_with_rates(&variant, options).await {
            Ok(document) => return Ok(document),
            Err(diagnostic) => {
                first.get_or_insert(diagnostic);
            }
        }
    }
    Err(first.expect("the raw variant is always tried first"))
}

/// Wraps text in ANSI styles only when the output is a color terminal.
struct Painter {
    enabled: bool,
}

impl Painter {
    const RESET: &'static str = "\x1b[0m";
    const BOLD: &'static str = "\x1b[1m";
    const DIM: &'static str = "\x1b[2m";
    const CYAN: &'static str = "\x1b[36m";

    fn from_env() -> Self {
        Self {
            enabled: io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none(),
        }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.enabled {
            format!("{code}{text}{}", Self::RESET)
        } else {
            text.to_owned()
        }
    }

    fn bold(&self, text: &str) -> String {
        self.paint(Self::BOLD, text)
    }

    fn dim(&self, text: &str) -> String {
        self.paint(Self::DIM, text)
    }

    fn cyan(&self, text: &str) -> String {
        self.paint(Self::CYAN, text)
    }
}

/// A REPL `:` command.
#[derive(Debug, PartialEq, Eq)]
enum ReplCommand {
    Clear,
    ClearHistory,
    Help,
    History,
    Quit,
}

impl ReplCommand {
    /// Returns the command for a source line, if it is one.
    fn parse(source: &str) -> Option<Self> {
        Some(match source {
            ":clear" => Self::Clear,
            ":clear-history" => Self::ClearHistory,
            ":help" => Self::Help,
            ":history" => Self::History,
            ":quit" | ":q" | ":exit" => Self::Quit,
            _ => return None,
        })
    }

    /// Returns every completion candidate, aliases included.
    fn completion_names() -> &'static [&'static str] {
        &[
            ":clear",
            ":clear-history",
            ":help",
            ":history",
            ":exit",
            ":quit",
        ]
    }
}

/// REPL helper that gives completion and a live result hint.
///
/// Rustyline helper callbacks only receive `&self`, so the helper keeps its
/// own copy of the session and context. The code syncs the copy after every
/// line.
#[derive(Clone)]
struct ReplHelper {
    session: CalculationSession,
    context: CalculationContext,
    color: bool,
}

impl ReplHelper {
    fn sync(&mut self, session: &CalculationSession, context: &CalculationContext) {
        self.session = session.clone();
        self.context = context.clone();
    }

    /// Returns completion candidates for the word at the cursor.
    fn candidates(&self, prefix: &str) -> Vec<String> {
        if prefix.starts_with(':') {
            return ReplCommand::completion_names()
                .iter()
                .filter(|command| command.starts_with(prefix))
                .map(|command| (*command).to_owned())
                .collect();
        }
        let lowered = prefix.to_ascii_lowercase();
        let mut matches: Vec<String> = self
            .session
            .variable_names()
            .filter(|name| name.starts_with(&lowered))
            .map(str::to_owned)
            .collect();
        matches.extend(
            Function::canonical_names()
                .filter(|name| name.starts_with(&lowered))
                .map(str::to_owned),
        );
        matches.extend(
            Currency::all()
                .iter()
                .map(|currency| currency.code())
                .filter(|code| code.to_ascii_lowercase().starts_with(&lowered))
                .map(str::to_owned),
        );
        matches.sort();
        matches.dedup();
        matches
    }

    /// Returns the live result hint for a complete line, if it evaluates.
    fn live_hint(&self, line: &str) -> Option<String> {
        let source = line.trim();
        if source.is_empty()
            || source.starts_with(':')
            || source.starts_with('!')
            || source.ends_with('=')
        {
            return None;
        }
        let mut session = self.session.clone();
        session
            .calculate(source, &self.context)
            .ok()
            .map(|calculation| format!(" = {}", calculation.output))
    }
}

impl rustyline::Helper for ReplHelper {}

impl Validator for ReplHelper {}

impl Highlighter for ReplHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        if self.color {
            format!("{}{hint}{}", "\x1b[2m", "\x1b[0m").into()
        } else {
            hint.into()
        }
    }
}

impl Hinter for ReplHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _context: &rustyline::Context<'_>) -> Option<String> {
        (pos >= line.len()).then(|| self.live_hint(line)).flatten()
    }
}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _context: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let prefix = line[..pos].rsplit(char::is_whitespace).next().unwrap_or("");
        if prefix.is_empty() {
            return Ok((pos, Vec::new()));
        }
        Ok((
            pos - prefix.len(),
            self.candidates(prefix)
                .into_iter()
                .map(|candidate| Pair {
                    display: candidate.clone(),
                    replacement: candidate,
                })
                .collect(),
        ))
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
    context.temporal.literal_zone_codes = options.literal_zone_codes;
    install_time_zones(&mut context);
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
    let painter = Painter::from_env();
    let prompt = painter.cyan("> ");
    let mut session = CalculationSession::new();
    let history_path = options.history.then(history::default_path);
    let mut editor = if interactive {
        let helper = ReplHelper {
            session: session.clone(),
            context: context.clone(),
            color: painter.enabled,
        };
        match ReadlineConfig::builder()
            .max_history_size(1_000)
            .and_then(|builder| builder.history_ignore_dups(true))
            .and_then(|builder| Editor::<ReplHelper, FileHistory>::with_config(builder.build()))
        {
            Ok(mut editor) => {
                editor.set_helper(Some(helper));
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
        if let Some(helper) = editor.as_mut().and_then(Editor::helper_mut) {
            helper.sync(&session, &context);
        }
        let source = if let Some(editor) = &mut editor {
            match editor.readline(&prompt) {
                Ok(source) => source,
                Err(ReadlineError::Interrupted) => break,
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
        match ReplCommand::parse(source) {
            Some(ReplCommand::Quit) => break,
            Some(ReplCommand::Clear) => {
                session.clear();
                continue;
            }
            Some(ReplCommand::ClearHistory) => {
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
            Some(ReplCommand::History) => {
                for (index, entry) in transient_history.iter().enumerate() {
                    println!("{}  {entry}", painter.dim(&(index + 1).to_string()));
                }
                continue;
            }
            Some(ReplCommand::Help) => {
                print_repl_help(&painter);
                continue;
            }
            None => {}
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

        let mut first_diagnostic = None;
        let mut result = None;
        for variant in surface_variants(source) {
            let mut entropy = OsEntropy::default();
            let mut outcome = session.calculate_with_entropy(&variant, &context, &mut entropy);
            if outcome
                .as_ref()
                .is_err_and(|error| error.kind() == DiagnosticKind::MissingTemporalContext)
            {
                install_current_datetime(&mut context, timezone);
                entropy.rewind();
                outcome = session.calculate_with_entropy(&variant, &context, &mut entropy);
            }
            if !attempted_rates
                && outcome.as_ref().is_err_and(|error| {
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
                outcome = session.calculate_with_entropy(&variant, &context, &mut entropy);
            }
            match outcome {
                Ok(calculation) => {
                    result = Some(Ok(calculation));
                    break;
                }
                Err(diagnostic) => {
                    first_diagnostic.get_or_insert(diagnostic);
                }
            }
        }
        let result = result.unwrap_or_else(|| {
            Err(first_diagnostic.expect("the raw variant is always tried first"))
        });

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

fn print_repl_help(painter: &Painter) {
    let rows = [
        (":help", "show this help"),
        (":quit, :q, :exit", "leave the session"),
        (":clear", "reset variables and ans"),
        (":clear-history", "remove saved history"),
        (":history", "list past inputs"),
        ("!!", "repeat the previous input"),
        ("!n", "repeat input n from :history"),
    ];
    println!("{}", painter.bold("commands"));
    for (name, description) in rows {
        println!("  {}  {description}", painter.cyan(name));
    }
    println!();
    println!("{}", painter.bold("input"));
    println!("  {}", painter.dim("press tab to complete words"));
    println!(
        "  {}",
        painter.dim("a dim = result hint appears as you type")
    );
    println!();
    println!("{}", painter.bold("examples"));
    for example in ["15% of 240", "100 eur to jpy", "hotel = $199 * 3"] {
        println!("  {example}");
    }
}

fn readline_history(editor: &Editor<ReplHelper, FileHistory>) -> Vec<String> {
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
    context.temporal.literal_zone_codes = options.literal_zone_codes;
    install_time_zones(&mut context);

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

fn install_time_zones(context: &mut CalculationContext) {
    match build_time_zone_snapshot() {
        Ok(snapshot) => context.temporal.time_zones = Some(snapshot),
        Err(error) => eprintln!("warning: could not load time-zone data: {error}"),
    }
}

fn build_time_zone_snapshot() -> Result<TimeZoneSnapshot, String> {
    let database = jiff::tz::db();
    // Calcite's year range is 1..=9999, but tzdb projects current DST rules far into the
    // future and materializing every projected transition makes startup unusably slow. A
    // generous window keeps the snapshot small; outside it resolution reports OutOfRange.
    const SNAPSHOT_FROM_YEAR: i16 = 1800;
    const SNAPSHOT_UNTIL_YEAR: i16 = 2200;
    let valid_from = civil::date(SNAPSHOT_FROM_YEAR, 1, 1)
        .to_zoned(TimeZone::UTC)
        .expect("the snapshot range is inside Jiff's supported range")
        .timestamp();
    let valid_until = civil::date(SNAPSHOT_UNTIL_YEAR, 1, 1)
        .to_zoned(TimeZone::UTC)
        .expect("the snapshot range is inside Jiff's supported range")
        .timestamp();
    // Abbreviations are collected only from periods in recent use: tzdb
    // reuses letter codes across decades (Manila called its 1976 daylight
    // saving `PDT`), and those one-offs must not make current codes
    // ambiguous. A two-year window still captures both DST phases.
    let codes_from = Timestamp::new(Timestamp::now().as_second() - 63_115_200, 0)
        .map_err(|error| format!("could not compute the abbreviation window: {error}"))?;
    let range_start = utc_date_time(valid_from)
        .ok_or_else(|| "the snapshot range does not fit Calcite's supported dates".to_owned())?;
    let range_end = utc_date_time(valid_until)
        .ok_or_else(|| "the snapshot range does not fit Calcite's supported dates".to_owned())?;

    // Per abbreviation: the offsets it names, the zones using each offset,
    // and which zones observe daylight saving.
    #[derive(Default)]
    struct CodeUse {
        standard: BTreeMap<i32, BTreeSet<String>>,
        daylight: BTreeMap<i32, BTreeSet<String>>,
        observes_dst: BTreeSet<String>,
    }
    let mut code_uses: BTreeMap<String, CodeUse> = BTreeMap::new();
    let mut record_code =
        |abbreviation: &str, offset_seconds: i32, daylight: bool, zone_name: &str| {
            let use_ = code_uses
                .entry(abbreviation.to_ascii_uppercase())
                .or_default();
            let producers = if daylight {
                &mut use_.daylight
            } else {
                &mut use_.standard
            }
            .entry(offset_seconds)
            .or_default();
            producers.insert(zone_name.to_owned());
            if daylight {
                use_.observes_dst.insert(zone_name.to_owned());
            }
        };

    let mut zones = Vec::new();
    for name in database.available() {
        let zone = TimeZone::get(name.as_str())
            .map_err(|error| format!("could not load zone {}: {error}", name.as_str()))?;
        let initial = UtcOffset::from_seconds(zone.to_offset(valid_from).seconds())
            .ok_or_else(|| format!("the offset of {} is out of range", name.as_str()))?;
        // `preceding` walks backward through time, newest transition first.
        let mut transitions = Vec::new();
        let mut zone_records = Vec::new();
        for transition in zone.preceding(valid_until) {
            let instant = transition.timestamp();
            if instant <= valid_from {
                break;
            }
            let Some(datetime) = utc_date_time(instant) else {
                break;
            };
            let offset_after = UtcOffset::from_seconds(transition.offset().seconds())
                .ok_or_else(|| format!("the offset of {} is out of range", name.as_str()))?;
            if instant >= codes_from {
                zone_records.push((
                    transition.abbreviation().to_owned(),
                    transition.offset().seconds(),
                    transition.dst() == jiff::tz::Dst::Yes,
                ));
            }
            transitions.push(TimeZoneTransition::new(datetime, offset_after));
        }
        transitions.reverse();
        // Zones without transitions in the code window (like Asia/Tokyo, which
        // has been UTC+9 since 1951) still need their abbreviation recorded.
        let current = zone.to_offset_info(codes_from);
        zone_records.push((
            current.abbreviation().to_owned(),
            current.offset().seconds(),
            current.dst() == jiff::tz::Dst::Yes,
        ));
        for (abbreviation, offset_seconds, daylight) in zone_records {
            record_code(&abbreviation, offset_seconds, daylight, name.as_str());
        }
        zones.push(
            TimeZoneDefinition::new(
                name.as_str().to_owned(),
                range_start,
                range_end,
                initial,
                transitions,
            )
            .map_err(|error| format!("zone {} is invalid: {error}", name.as_str()))?,
        );
    }

    let zone_names: BTreeSet<String> = zones
        .iter()
        .map(TimeZoneDefinition::name)
        .map(str::to_owned)
        .collect();
    let mut codes = Vec::new();
    for (code, use_) in &code_uses {
        // Only letter codes are typable as words; tzdb also reports strings
        // like `+05` or `LMT` for offset-only periods.
        if code.len() < 2 || code.len() > 6 || !code.bytes().all(|byte| byte.is_ascii_alphabetic())
        {
            continue;
        }
        let literal_candidates = if use_.standard.is_empty() {
            &use_.daylight
        } else {
            &use_.standard
        };
        let entry = if literal_candidates.len() > 1 {
            // The abbreviation names several offsets (CST, IST, ...): offer
            // one representative zone per offset instead of guessing.
            TimeZoneCode::Ambiguous {
                candidates: literal_candidates
                    .keys()
                    .map(|offset| {
                        format!(
                            "UTC{} ({})",
                            UtcOffset::from_seconds(*offset)
                                .map(|offset| offset.to_string())
                                .unwrap_or_default(),
                            representative_zone_name(
                                literal_candidates[offset].iter(),
                                &use_.observes_dst,
                            )
                        )
                    })
                    .collect(),
            }
        } else {
            let Some(&offset) = literal_candidates.keys().next() else {
                continue;
            };
            let Some(literal_offset) = UtcOffset::from_seconds(offset) else {
                continue;
            };
            let producers: Vec<&String> = literal_candidates[&offset].iter().collect();
            let representative = zone_names
                .iter()
                .find(|zone| zone.eq_ignore_ascii_case(code))
                .cloned()
                .or_else(|| {
                    Some(
                        representative_zone_name(producers.iter().copied(), &use_.observes_dst)
                            .to_owned(),
                    )
                })
                .unwrap_or_else(|| code.clone());
            TimeZoneCode::Resolved {
                literal_offset,
                representative_zone: representative,
            }
        };
        codes.push((code.clone(), entry));
    }

    TimeZoneSnapshot::new("jiff-tzdb", zones, codes)
        .map_err(|error| format!("the time-zone snapshot is invalid: {error}"))
}

/// Picks a zone to display for an abbreviation: one that observes daylight
/// saving when any producer does (so `AEST` follows Sydney rather than the
/// DST-less Brisbane), and a readable path-style name (`America/Chicago`
/// rather than the legacy `CST6CDT`) over short link names.
fn representative_zone_name<'a>(
    producers: impl Iterator<Item = &'a String>,
    observes_dst: &BTreeSet<String>,
) -> &'a str {
    producers
        .min_by_key(|name| {
            (
                !observes_dst.contains(*name),
                !name.contains('/'),
                name.matches('/').count(),
                name.len(),
                name.to_owned(),
            )
        })
        .map(String::as_str)
        .unwrap_or("unknown")
}

fn utc_date_time(timestamp: Timestamp) -> Option<OffsetDateTime> {
    let zoned = timestamp.to_zoned(TimeZone::UTC);
    let date = Date::new(
        zoned.year().into(),
        u8::try_from(zoned.month()).ok()?,
        u8::try_from(zoned.day()).ok()?,
    )?;
    let time = ClockTime::with_nanosecond(
        u8::try_from(zoned.hour()).ok()?,
        u8::try_from(zoned.minute()).ok()?,
        u8::try_from(zoned.second()).ok()?,
        u32::try_from(zoned.subsec_nanosecond()).ok()?,
    )?;
    Some(OffsetDateTime::new(date, time, UtcOffset::UTC))
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
    let cli = Cli::parse();
    let defaults = config::load(cli.config.clone())?;

    let number_format = match cli.locale.as_deref() {
        Some(value) => config::parse_locale(value)?,
        None => defaults.number_format,
    };
    let requested_number_mode = match cli.format.as_deref() {
        Some(value) => config::parse_format(value)?,
        None => defaults.number_mode,
    };
    let precision = match cli.precision.as_deref() {
        Some(value) => value
            .parse::<u8>()
            .ok()
            .filter(|value| (1..=100).contains(value))
            .ok_or_else(|| "--precision requires an integer from 1 through 100".to_owned())?,
        None => defaults.precision,
    };
    let date_order = match cli.date_order.as_deref() {
        Some(value) => Some(config::parse_date_order(value)?),
        None => defaults.date_order,
    };
    let default_currency = match cli.currency.as_deref() {
        Some(value) => Currency::from_code(value)
            .ok_or_else(|| "--currency requires an ISO 4217 code".to_owned())?,
        None => defaults.default_currency,
    };
    let timezone = match cli.timezone.as_deref() {
        Some(value) => {
            TimeZone::get(value).map_err(|error| format!("invalid timezone '{value}': {error}"))?;
            Some(value.to_owned())
        }
        None => defaults.timezone,
    };
    let rate_provider = match cli.rate_provider.as_deref() {
        Some(value) => config::parse_provider(value)?,
        None => defaults.rate_provider,
    };
    let output = if cli.json {
        OutputMode::Json
    } else if cli.plain {
        OutputMode::Plain
    } else {
        OutputMode::Text
    };
    let inputs = [
        cli.repl,
        cli.eval.is_some(),
        cli.file.is_some(),
        !cli.expression.is_empty(),
    ]
    .iter()
    .filter(|specified| **specified)
    .count();
    if inputs > 1 {
        return Err("only one input source may be specified".into());
    }

    let action = if cli.repl {
        Action::Repl
    } else if let Some(expression) = cli.eval {
        Action::Calculate(expression)
    } else if let Some(path) = cli.file {
        Action::Calculate(
            fs::read_to_string(&path)
                .map_err(|error| format!("could not read {}: {error}", path.display()))?,
        )
    } else if !cli.expression.is_empty() {
        Action::Calculate(cli.expression.join(" "))
    } else if io::stdin().is_terminal() {
        Action::Repl
    } else {
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|error| error.to_string())?;
        Action::Calculate(source.trim().to_owned())
    };

    if let Some(argument) = cli
        .expression
        .iter()
        .find(|argument| argument.starts_with("--"))
    {
        return Err(format!("unknown option: {argument}"));
    }

    Ok(Options {
        action,
        refresh_rates: cli.refresh_rates,
        output,
        date_order,
        number_format,
        number_mode: requested_number_mode.resolve(precision),
        mark_approximate: cli.approximate || defaults.approximate,
        default_currency,
        custom_units: defaults.custom_units,
        timezone,
        rate_provider,
        history: defaults.history && !cli.no_history,
        literal_zone_codes: cli.literal_zone_codes || defaults.literal_zone_codes,
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
            "mean": canonical_number(&value.mean()),
            "outcomes": value
                .outcome_probabilities()
                .into_iter()
                .map(|(total, probability)| serde_json::json!({
                    "value": total,
                    "probability": canonical_number(probability.ratio()),
                }))
                .collect::<Vec<_>>(),
        }),
        Value::Coin(value) => textual_json("coin_side", value),
        Value::Date(value) => serde_json::json!({
            "type": "date",
            "value": value.to_string(),
            "year": value.year(),
            "month": value.month(),
            "day": value.day(),
        }),
        Value::Clock(value) => serde_json::json!({
            "type": "clock_time",
            "value": value.to_string(),
            "hour": value.hour(),
            "minute": value.minute(),
            "second": value.second(),
            "nanosecond": value.nanosecond(),
        }),
        Value::OffsetClock(value) => serde_json::json!({
            "type": "offset_clock_time",
            "value": value.to_string(),
            "hour": value.time().hour(),
            "minute": value.time().minute(),
            "second": value.time().second(),
            "nanosecond": value.time().nanosecond(),
            "offset_seconds": value.offset().seconds(),
        }),
        Value::ZonedClock(value) => serde_json::json!({
            "type": "zoned_clock_time",
            "value": value.to_string(),
            "hour": value.time().hour(),
            "minute": value.time().minute(),
            "second": value.time().second(),
            "nanosecond": value.time().nanosecond(),
            "zone": value.zone(),
        }),
        Value::OffsetDateTime(value) => serde_json::json!({
            "type": "offset_date_time",
            "value": value.to_string(),
            "year": value.date().year(),
            "month": value.date().month(),
            "day": value.date().day(),
            "hour": value.time().hour(),
            "minute": value.time().minute(),
            "second": value.time().second(),
            "nanosecond": value.time().nanosecond(),
            "offset_seconds": value.offset().seconds(),
        }),
        Value::ZonedDateTime(value) => serde_json::json!({
            "type": "zoned_date_time",
            "value": value.to_string(),
            "year": value.datetime().date().year(),
            "month": value.datetime().date().month(),
            "day": value.datetime().date().day(),
            "hour": value.datetime().time().hour(),
            "minute": value.datetime().time().minute(),
            "second": value.datetime().time().second(),
            "nanosecond": value.datetime().time().nanosecond(),
            "offset_seconds": value.datetime().offset().seconds(),
            "zone": value.zone(),
        }),
        Value::TimeSpan(value) => serde_json::json!({
            "type": "time_span",
            "value": value.to_string(),
            "nanoseconds": value.nanoseconds().to_string(),
            "seconds": exact_seconds(value.nanoseconds()),
        }),
        Value::CalendarDuration(value) => serde_json::json!({
            "type": "calendar_duration",
            "value": value.to_string(),
            "months": value.total_months(),
            "unit": if matches!(value.unit(), CalendarDurationUnit::Year) { "years" } else { "months" },
        }),
        Value::CalendarSpan(value) => serde_json::json!({
            "type": "calendar_span",
            "value": value.to_string(),
            "months": value.total_months(),
            "days": value.remaining_days(),
        }),
        Value::WorkdayDuration(value) => serde_json::json!({
            "type": "workday_duration",
            "value": value.to_string(),
            "days": value.days(),
        }),
        Value::Weekday(value) => serde_json::json!({
            "type": "weekday",
            "value": value.to_string(),
            "index_from_monday": value.number_from_monday(),
        }),
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

fn exact_seconds(nanoseconds: i128) -> String {
    let sign = if nanoseconds < 0 { "-" } else { "" };
    let magnitude = nanoseconds.unsigned_abs();
    let seconds = magnitude / 1_000_000_000;
    let fraction = format!("{:09}", magnitude % 1_000_000_000);
    let fraction = fraction.trim_end_matches('0');
    if fraction.is_empty() {
        format!("{sign}{seconds}")
    } else {
        format!("{sign}{seconds}.{fraction}")
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn helper_with_line(line: &str) -> ReplHelper {
        let mut session = CalculationSession::new();
        session
            .calculate(line, &CalculationContext::default())
            .expect("seed line evaluates");
        ReplHelper {
            session,
            context: CalculationContext::default(),
            color: false,
        }
    }

    #[test]
    fn commands_parse_from_their_aliases() {
        assert_eq!(ReplCommand::parse(":quit"), Some(ReplCommand::Quit));
        assert_eq!(ReplCommand::parse(":q"), Some(ReplCommand::Quit));
        assert_eq!(ReplCommand::parse(":exit"), Some(ReplCommand::Quit));
        assert_eq!(
            ReplCommand::parse(":clear-history"),
            Some(ReplCommand::ClearHistory)
        );
        assert_eq!(ReplCommand::parse(":help"), Some(ReplCommand::Help));
        assert_eq!(ReplCommand::parse(":quitting"), None);
        assert_eq!(ReplCommand::parse("2 + 2"), None);
    }

    #[test]
    fn help_mentions_every_command() {
        for name in ReplCommand::completion_names() {
            assert!(name.starts_with(':'));
        }
        assert!(ReplCommand::completion_names().contains(&":help"));
        assert!(!ReplCommand::completion_names().contains(&":q"));
    }

    #[test]
    fn completion_matches_variables_functions_and_currencies() {
        let helper = helper_with_line("subtotal = 80");
        assert!(helper.candidates("sub").contains(&"subtotal".to_owned()));
        assert!(helper.candidates("sq").contains(&"sqrt".to_owned()));
        assert!(helper.candidates("eu").contains(&"EUR".to_owned()));
        assert!(helper.candidates("an").contains(&"ans".to_owned()));
        assert!(helper.candidates("zz").is_empty());
    }

    #[test]
    fn completion_matches_commands_after_colon() {
        let helper = helper_with_line("2 + 2");
        assert_eq!(helper.candidates(":q"), vec![":quit".to_owned()]);
        assert!(helper.candidates(":c").len() >= 2);
    }

    #[test]
    fn live_hints_show_results_but_skip_commands_and_entropy() {
        let helper = helper_with_line("x = 4");
        assert_eq!(helper.live_hint("x * 2"), Some(" = 8".to_owned()));
        assert_eq!(helper.live_hint("  2 + 2  "), Some(" = 4".to_owned()));
        assert_eq!(helper.live_hint(":quit"), None);
        assert_eq!(helper.live_hint("x ="), None);
        assert_eq!(helper.live_hint("invalid syntax here!!"), None);
        // Sampling needs entropy. A hint does not supply it.
        assert_eq!(helper.live_hint("flip a coin"), None);
    }
}
