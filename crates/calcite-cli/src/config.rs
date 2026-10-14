use std::{env, fs, path::PathBuf};

use calcite_core::{
    CalculationContext, Currency, CustomUnitRegistry, DateOrder, NumberFormat, Value,
    calculate_with_context,
};

use crate::{RequestedNumberMode, rates::ProviderChoice};

pub struct Config {
    pub number_format: NumberFormat,
    pub date_order: Option<DateOrder>,
    pub number_mode: RequestedNumberMode,
    pub precision: u8,
    pub approximate: bool,
    pub default_currency: Currency,
    pub timezone: Option<String>,
    pub rate_provider: ProviderChoice,
    pub history: bool,
    pub literal_zone_codes: bool,
    pub custom_units: CustomUnitRegistry,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            number_format: NumberFormat::default(),
            date_order: None,
            number_mode: RequestedNumberMode::Auto,
            precision: 10,
            approximate: false,
            default_currency: Currency::Usd,
            timezone: None,
            rate_provider: ProviderChoice::Fallback,
            history: true,
            literal_zone_codes: false,
            custom_units: CustomUnitRegistry::default(),
        }
    }
}

pub fn load(explicit_path: Option<PathBuf>) -> Result<Config, String> {
    let path = explicit_path.or_else(default_path);
    let Some(path) = path else {
        return Ok(Config::default());
    };
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(error) => return Err(format!("could not read {}: {error}", path.display())),
    };
    let value: serde_json::Value = serde_json::from_str(&source)
        .map_err(|error| format!("invalid configuration {}: {error}", path.display()))?;
    let object = value
        .as_object()
        .ok_or_else(|| format!("configuration {} must be a JSON object", path.display()))?;
    let mut config = Config::default();
    let mut custom_units = None;
    for (key, value) in object {
        match key.as_str() {
            "locale" => config.number_format = parse_locale(string(value, key)?)?,
            "date_order" => config.date_order = Some(parse_date_order(string(value, key)?)?),
            "format" => config.number_mode = parse_format(string(value, key)?)?,
            "precision" => {
                config.precision = value
                    .as_u64()
                    .and_then(|value| u8::try_from(value).ok())
                    .filter(|value| (1..=100).contains(value))
                    .ok_or_else(|| {
                        "configuration precision must be from 1 through 100".to_owned()
                    })?;
            }
            "approximate" => {
                config.approximate = value
                    .as_bool()
                    .ok_or_else(|| "configuration approximate must be true or false".to_owned())?;
            }
            "default_currency" => {
                config.default_currency =
                    Currency::from_code(string(value, key)?).ok_or_else(|| {
                        "configuration default_currency must be an ISO 4217 code".to_owned()
                    })?;
            }
            "timezone" => config.timezone = Some(string(value, key)?.to_owned()),
            "literal_zone_codes" => {
                config.literal_zone_codes = value.as_bool().ok_or_else(|| {
                    "configuration literal_zone_codes must be true or false".to_owned()
                })?;
            }
            "rate_provider" => config.rate_provider = parse_provider(string(value, key)?)?,
            "history" => {
                config.history = value
                    .as_bool()
                    .ok_or_else(|| "configuration history must be true or false".to_owned())?;
            }
            "custom_units" => {
                custom_units = Some(value.as_object().ok_or_else(|| {
                    "configuration custom_units must be an object of expression strings".to_owned()
                })?);
            }
            _ => return Err(format!("unknown configuration key: {key}")),
        }
    }
    if let Some(units) = custom_units {
        let mut context = CalculationContext {
            number_format: config.number_format,
            ..CalculationContext::default()
        };
        for (name, expression) in units {
            let expression = string(expression, name)?;
            let calculation = calculate_with_context(expression, &context)
                .map_err(|error| format!("invalid custom unit '{name}': {error}"))?;
            let Value::Unit(value) = calculation.value else {
                return Err(format!(
                    "invalid custom unit '{name}': definition must be a physical or data unit"
                ));
            };
            context
                .custom_units
                .define(
                    name,
                    value.magnitude().clone(),
                    value.compound_unit().clone(),
                )
                .map_err(|error| format!("invalid custom unit '{name}': {error}"))?;
        }
        config.custom_units = context.custom_units;
    }
    Ok(config)
}

pub fn default_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("CALCITE_CONFIG") {
        return (!path.is_empty()).then(|| PathBuf::from(path));
    }
    if let Some(directory) = env::var_os("XDG_CONFIG_HOME").filter(|path| !path.is_empty()) {
        return Some(PathBuf::from(directory).join("calcite/config.json"));
    }
    env::var_os("HOME")
        .filter(|path| !path.is_empty())
        .map(|home| PathBuf::from(home).join(".config/calcite/config.json"))
}

pub fn parse_locale(value: &str) -> Result<NumberFormat, String> {
    match value.to_ascii_lowercase().as_str() {
        "en" | "en-us" | "en-gb" | "decimal-point" => Ok(NumberFormat::decimal_point()),
        "de" | "de-de" | "fr" | "fr-fr" | "decimal-comma" => Ok(NumberFormat::decimal_comma()),
        _ => Err("locale must be en-US, en-GB, de-DE, or fr-FR".to_owned()),
    }
}

pub fn parse_date_order(value: &str) -> Result<DateOrder, String> {
    match value.to_ascii_lowercase().as_str() {
        "dmy" => Ok(DateOrder::DayMonthYear),
        "mdy" => Ok(DateOrder::MonthDayYear),
        "ymd" => Ok(DateOrder::YearMonthDay),
        _ => Err("date order must be dmy, mdy, or ymd".into()),
    }
}

pub fn parse_format(value: &str) -> Result<RequestedNumberMode, String> {
    match value.to_ascii_lowercase().as_str() {
        "auto" => Ok(RequestedNumberMode::Auto),
        "plain" | "unformatted" => Ok(RequestedNumberMode::Plain),
        "exact" => Ok(RequestedNumberMode::Exact),
        "fraction" => Ok(RequestedNumberMode::Fraction),
        "mixed" => Ok(RequestedNumberMode::Mixed),
        "decimal" => Ok(RequestedNumberMode::Decimal),
        "significant" => Ok(RequestedNumberMode::Significant),
        "scientific" => Ok(RequestedNumberMode::Scientific),
        _ => Err(
            "format must be auto, plain, exact, fraction, mixed, decimal, significant, or scientific"
                .into(),
        ),
    }
}

pub fn parse_provider(value: &str) -> Result<ProviderChoice, String> {
    match value.to_ascii_lowercase().as_str() {
        "fallback" | "auto" => Ok(ProviderChoice::Fallback),
        "frankfurter" => Ok(ProviderChoice::Frankfurter),
        "ecb" => Ok(ProviderChoice::Ecb),
        _ => Err("rate provider must be fallback, frankfurter, or ecb".into()),
    }
}

fn string<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    value
        .as_str()
        .ok_or_else(|| format!("configuration {key} must be a string"))
}
