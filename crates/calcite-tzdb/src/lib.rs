use std::collections::{BTreeMap, BTreeSet};

use calcite_core::{
    ClockTime, Date, OffsetDateTime, TimeZoneCode, TimeZoneDefinition, TimeZoneSnapshot,
    TimeZoneTransition, UtcOffset,
};
use jiff::{Timestamp, civil, tz::TimeZone};

pub fn build_time_zone_snapshot() -> Result<TimeZoneSnapshot, String> {
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
