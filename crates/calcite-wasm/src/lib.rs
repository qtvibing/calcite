use calcite_core::{
    Calculation, CalculationContext, CalculationSession, ClockTime, Date, Diagnostic,
    EntropySource, OffsetDateTime, UtcOffset,
};
use wasm_bindgen::prelude::*;

struct JsEntropy;

impl EntropySource for JsEntropy {
    fn next_u64(&mut self) -> Option<u64> {
        let high = (js_sys::Math::random() * 4_294_967_296.0) as u32;
        let low = (js_sys::Math::random() * 4_294_967_296.0) as u32;
        Some((u64::from(high) << 32) | u64::from(low))
    }
}

#[wasm_bindgen]
pub fn calculate(source: &str) -> Result<JsValue, JsValue> {
    let context = browser_context();
    calcite_core::calculate_with_context_and_entropy(source, &context, &mut JsEntropy)
        .map_err(error_json)
        .and_then(|calculation| result_json(&calculation))
}

#[wasm_bindgen]
pub struct Session {
    session: CalculationSession,
    context: CalculationContext,
}

#[wasm_bindgen]
impl Session {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            session: CalculationSession::new(),
            context: browser_context(),
        }
    }

    pub fn calculate(&mut self, source: &str) -> Result<JsValue, JsValue> {
        self.session
            .calculate_with_entropy(source, &self.context, &mut JsEntropy)
            .map_err(error_json)
            .and_then(|calculation| result_json(&calculation))
    }

    pub fn clear(&mut self) {
        self.session.clear();
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

fn browser_context() -> CalculationContext {
    let mut context = CalculationContext::default();
    if let Some(now) = current_datetime() {
        context.temporal.default_offset = now.offset();
        context.temporal.current_datetime = Some(now);
    }
    #[cfg(feature = "tzdb")]
    {
        context.temporal.time_zones = time_zone_snapshot().cloned();
    }
    context
}

fn current_datetime() -> Option<OffsetDateTime> {
    let now = js_sys::Date::new_0();
    let date = Date::new(
        now.get_full_year() as i32,
        (now.get_month() + 1) as u8,
        now.get_date() as u8,
    )?;
    let time = ClockTime::with_nanosecond(
        now.get_hours() as u8,
        now.get_minutes() as u8,
        now.get_seconds() as u8,
        now.get_milliseconds() * 1_000_000,
    )?;
    let offset = UtcOffset::from_seconds(-(now.get_timezone_offset() as i32) * 60)?;
    Some(OffsetDateTime::new(date, time, offset))
}

#[cfg(feature = "tzdb")]
fn time_zone_snapshot() -> Option<&'static calcite_core::TimeZoneSnapshot> {
    use std::sync::OnceLock;

    static SNAPSHOT: OnceLock<Option<calcite_core::TimeZoneSnapshot>> = OnceLock::new();
    SNAPSHOT
        .get_or_init(|| calcite_tzdb::build_time_zone_snapshot().ok())
        .as_ref()
}

fn result_json(calculation: &Calculation) -> Result<JsValue, JsValue> {
    parse_json(&serde_json::json!({
        "ok": true,
        "output": calculation.output,
        "value": calculation.value.to_json(),
    }))
}

fn error_json(diagnostic: Diagnostic) -> JsValue {
    let span = diagnostic.span();
    let json = serde_json::json!({
        "ok": false,
        "error": {
            "kind": format!("{:?}", diagnostic.kind()),
            "message": diagnostic.to_string(),
            "span": { "start": span.start, "end": span.end },
        },
    });
    parse_json(&json).unwrap_or(JsValue::NULL)
}

fn parse_json(value: &serde_json::Value) -> Result<JsValue, JsValue> {
    let text =
        serde_json::to_string(value).map_err(|error| JsValue::from_str(&error.to_string()))?;
    js_sys::JSON::parse(&text)
}
