use std::{collections::BTreeMap, fmt};

use crate::{CalendarDuration, ClockTime, Date, OffsetDateTime, UtcOffset};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalTimeResolution {
    Reject,
    Earlier,
    Later,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeZoneTransition {
    instant: OffsetDateTime,
    offset_after: UtcOffset,
}

impl TimeZoneTransition {
    pub const fn new(instant: OffsetDateTime, offset_after: UtcOffset) -> Self {
        Self {
            instant,
            offset_after,
        }
    }

    pub const fn instant(self) -> OffsetDateTime {
        self.instant
    }

    pub const fn offset_after(self) -> UtcOffset {
        self.offset_after
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeZoneDefinition {
    name: String,
    valid_from: OffsetDateTime,
    valid_until: OffsetDateTime,
    initial_offset: UtcOffset,
    transitions: Vec<TimeZoneTransition>,
}

impl TimeZoneDefinition {
    pub fn new(
        name: impl Into<String>,
        valid_from: OffsetDateTime,
        valid_until: OffsetDateTime,
        initial_offset: UtcOffset,
        transitions: impl IntoIterator<Item = TimeZoneTransition>,
    ) -> Result<Self, TimeZoneError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(TimeZoneError::InvalidDefinition);
        }
        if instant_cmp(valid_from, valid_until).is_ge() {
            return Err(TimeZoneError::InvalidDefinition);
        }
        let transitions = transitions.into_iter().collect::<Vec<_>>();
        let mut previous = None;
        for transition in &transitions {
            if instant_cmp(transition.instant, valid_from).is_lt()
                || !instant_cmp(transition.instant, valid_until).is_lt()
                || previous
                    .is_some_and(|previous| !instant_cmp(previous, transition.instant).is_lt())
            {
                return Err(TimeZoneError::InvalidDefinition);
            }
            previous = Some(transition.instant);
        }
        Ok(Self {
            name,
            valid_from,
            valid_until,
            initial_offset,
            transitions,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn valid_from(&self) -> OffsetDateTime {
        self.valid_from
    }

    pub const fn valid_until(&self) -> OffsetDateTime {
        self.valid_until
    }

    pub const fn initial_offset(&self) -> UtcOffset {
        self.initial_offset
    }

    pub fn transitions(&self) -> &[TimeZoneTransition] {
        &self.transitions
    }

    /// The wall time in this zone at the given instant.
    pub fn zoned_at_instant(
        &self,
        instant: OffsetDateTime,
    ) -> Result<ZonedDateTime, TimeZoneError> {
        let offset = self.offset_at(instant)?;
        let datetime = instant
            .converted_to(offset)
            .ok_or(TimeZoneError::OutOfRange)?;
        Ok(ZonedDateTime {
            datetime,
            zone: self.name.clone(),
        })
    }

    /// The instant this zone assigns to a wall time, applying the given
    /// policy for times that do not exist or occur twice.
    pub fn zoned_from_local(
        &self,
        date: Date,
        time: ClockTime,
        resolution: LocalTimeResolution,
    ) -> Result<ZonedDateTime, TimeZoneError> {
        let mut had_in_range_candidate = false;
        let mut candidates = self
            .possible_offsets()
            .into_iter()
            .map(|offset| OffsetDateTime::new(date, time, offset))
            .filter(|candidate| match self.offset_at(*candidate) {
                Ok(offset) => {
                    had_in_range_candidate = true;
                    offset == candidate.offset()
                }
                Err(_) => false,
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| instant_cmp(*left, *right));
        candidates.dedup_by(|left, right| instant_cmp(*left, *right).is_eq());
        let datetime = match candidates.as_slice() {
            [] if !had_in_range_candidate => return Err(TimeZoneError::OutOfRange),
            [] => return Err(TimeZoneError::NonexistentLocalTime),
            [datetime] => *datetime,
            candidates => match resolution {
                LocalTimeResolution::Reject => return Err(TimeZoneError::AmbiguousLocalTime),
                LocalTimeResolution::Earlier => candidates[0],
                LocalTimeResolution::Later => candidates[candidates.len() - 1],
            },
        };
        Ok(ZonedDateTime {
            datetime,
            zone: self.name.clone(),
        })
    }

    fn offset_at(&self, instant: OffsetDateTime) -> Result<UtcOffset, TimeZoneError> {
        if instant_cmp(instant, self.valid_from).is_lt()
            || !instant_cmp(instant, self.valid_until).is_lt()
        {
            return Err(TimeZoneError::OutOfRange);
        }
        let mut offset = self.initial_offset;
        for transition in &self.transitions {
            if instant_cmp(transition.instant, instant).is_gt() {
                break;
            }
            offset = transition.offset_after;
        }
        Ok(offset)
    }

    fn possible_offsets(&self) -> Vec<UtcOffset> {
        let mut offsets = vec![self.initial_offset];
        for transition in &self.transitions {
            if !offsets.contains(&transition.offset_after) {
                offsets.push(transition.offset_after);
            }
        }
        offsets
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeZoneSnapshot {
    version: String,
    zones: BTreeMap<String, TimeZoneDefinition>,
    codes: BTreeMap<String, TimeZoneCode>,
}

/// How a time-zone abbreviation such as `CET` or `CST` should be interpreted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimeZoneCode {
    /// The abbreviation has a single meaning.
    Resolved {
        /// The fixed offset the abbreviation literally names.
        literal_offset: UtcOffset,
        /// The IANA zone whose rules apply when abbreviations are resolved
        /// against their region instead of literally.
        representative_zone: String,
    },
    /// The abbreviation names several offsets or regions; the zones that
    /// produce it are offered as candidates.
    Ambiguous { candidates: Vec<String> },
}

/// The meaning of a time-zone specifier written by the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneSpecifier {
    Zone,
    Abbreviation,
    Ambiguous,
}

impl TimeZoneSnapshot {
    pub fn new(
        version: impl Into<String>,
        zones: impl IntoIterator<Item = TimeZoneDefinition>,
        codes: impl IntoIterator<Item = (String, TimeZoneCode)>,
    ) -> Result<Self, TimeZoneError> {
        let version = version.into();
        if version.trim().is_empty() {
            return Err(TimeZoneError::InvalidDefinition);
        }
        let mut indexed = BTreeMap::new();
        for zone in zones {
            let key = zone.name.to_ascii_lowercase();
            if indexed.insert(key, zone).is_some() {
                return Err(TimeZoneError::DuplicateZone);
            }
        }
        let mut indexed_codes = BTreeMap::new();
        for (code, entry) in codes {
            let key = code.to_ascii_lowercase();
            if indexed_codes.insert(key, entry).is_some() {
                return Err(TimeZoneError::DuplicateZone);
            }
        }
        Ok(Self {
            version,
            zones: indexed,
            codes: indexed_codes,
        })
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn zones(&self) -> impl Iterator<Item = &TimeZoneDefinition> {
        self.zones.values()
    }

    /// Classifies a specifier without resolving it to a definition, so the
    /// parser can decide whether a word is a time-zone specifier.
    pub fn specifier_kind(&self, specifier: &str) -> Option<ZoneSpecifier> {
        self.resolve_specifier(specifier)
            .map(|resolution| match resolution {
                ZoneResolution::Zone(_) => ZoneSpecifier::Zone,
                ZoneResolution::Abbreviation(_) => ZoneSpecifier::Abbreviation,
                ZoneResolution::Ambiguous(_) => ZoneSpecifier::Ambiguous,
            })
    }

    /// Resolves a user-written specifier: an IANA zone name, a time-zone
    /// abbreviation such as `CET`, or the trailing segment of a zone name
    /// such as `paris` for `Europe/Paris`.
    pub fn resolve_specifier(&self, specifier: &str) -> Option<ZoneResolution<'_>> {
        let key = specifier.to_ascii_lowercase();
        // Codes are checked before zone names: several codes (CET, EST, ...)
        // are also legacy zone names, and their abbreviation entry decides
        // whether they resolve literally or to their representative zone.
        if let Some(entry) = self.codes.get(&key) {
            return Some(match entry {
                TimeZoneCode::Resolved {
                    literal_offset,
                    representative_zone,
                } => ZoneResolution::Abbreviation(TimeZoneAbbreviation {
                    literal_offset: *literal_offset,
                    representative_zone: representative_zone.clone(),
                }),
                TimeZoneCode::Ambiguous { candidates } => {
                    ZoneResolution::Ambiguous(candidates.iter().map(String::as_str).collect())
                }
            });
        }
        if let Some(definition) = self.zones.get(&key) {
            return Some(ZoneResolution::Zone(definition));
        }
        let hits = self
            .zones
            .values()
            .filter(|definition| {
                definition
                    .name
                    .rsplit('/')
                    .next()
                    .is_some_and(|segment| same_city(segment, specifier))
            })
            .collect::<Vec<_>>();
        match hits.as_slice() {
            [] => None,
            [definition] => Some(ZoneResolution::Zone(definition)),
            _ => Some(ZoneResolution::Ambiguous(
                hits.iter()
                    .map(|definition| definition.name.as_str())
                    .collect(),
            )),
        }
    }

    pub fn zone(&self, name: &str) -> Option<&TimeZoneDefinition> {
        self.zones.get(&name.to_ascii_lowercase())
    }

    pub fn at_instant(
        &self,
        zone: &str,
        instant: OffsetDateTime,
    ) -> Result<ZonedDateTime, TimeZoneError> {
        self.zone(zone)
            .ok_or(TimeZoneError::UnknownZone)?
            .zoned_at_instant(instant)
    }

    pub fn resolve_local(
        &self,
        zone: &str,
        date: Date,
        time: ClockTime,
        resolution: LocalTimeResolution,
    ) -> Result<ZonedDateTime, TimeZoneError> {
        self.zone(zone)
            .ok_or(TimeZoneError::UnknownZone)?
            .zoned_from_local(date, time, resolution)
    }
}

/// A time-zone abbreviation resolved from the snapshot's code table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeZoneAbbreviation {
    literal_offset: UtcOffset,
    representative_zone: String,
}

impl TimeZoneAbbreviation {
    pub const fn new(literal_offset: UtcOffset, representative_zone: String) -> Self {
        Self {
            literal_offset,
            representative_zone,
        }
    }

    pub const fn literal_offset(&self) -> UtcOffset {
        self.literal_offset
    }

    pub fn representative_zone(&self) -> &str {
        &self.representative_zone
    }
}

/// A fully resolved meaning of a user-written specifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZoneResolution<'a> {
    Zone(&'a TimeZoneDefinition),
    Abbreviation(TimeZoneAbbreviation),
    Ambiguous(Vec<&'a str>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZonedClockTime {
    time: ClockTime,
    zone: String,
}

impl ZonedClockTime {
    pub fn new(time: ClockTime, zone: impl Into<String>) -> Self {
        Self {
            time,
            zone: zone.into(),
        }
    }

    pub const fn time(&self) -> ClockTime {
        self.time
    }

    pub fn zone(&self) -> &str {
        &self.zone
    }
}

impl fmt::Display for ZonedClockTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.time, self.zone)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZonedDateTime {
    datetime: OffsetDateTime,
    zone: String,
}

impl ZonedDateTime {
    pub const fn datetime(&self) -> OffsetDateTime {
        self.datetime
    }

    pub fn zone(&self) -> &str {
        &self.zone
    }

    pub fn add_nanoseconds(
        &self,
        snapshot: &TimeZoneSnapshot,
        nanoseconds: i128,
    ) -> Result<Self, TimeZoneError> {
        let instant = self
            .datetime
            .add_nanoseconds(nanoseconds)
            .ok_or(TimeZoneError::OutOfRange)?;
        snapshot.at_instant(&self.zone, instant)
    }

    pub fn add_calendar_days(
        &self,
        snapshot: &TimeZoneSnapshot,
        days: i64,
        resolution: LocalTimeResolution,
    ) -> Result<Self, TimeZoneError> {
        let date = self
            .datetime
            .date()
            .add_days(days)
            .ok_or(TimeZoneError::OutOfRange)?;
        snapshot.resolve_local(&self.zone, date, self.datetime.time(), resolution)
    }

    pub fn add_calendar_duration(
        &self,
        snapshot: &TimeZoneSnapshot,
        duration: CalendarDuration,
        resolution: LocalTimeResolution,
    ) -> Result<Self, TimeZoneError> {
        let date = self
            .datetime
            .date()
            .add_months(duration.total_months())
            .ok_or(TimeZoneError::OutOfRange)?;
        snapshot.resolve_local(&self.zone, date, self.datetime.time(), resolution)
    }
}

impl fmt::Display for ZonedDateTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}[{}]", self.datetime, self.zone)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeZoneError {
    InvalidDefinition,
    DuplicateZone,
    UnknownZone,
    OutOfRange,
    AmbiguousLocalTime,
    NonexistentLocalTime,
}

impl fmt::Display for TimeZoneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidDefinition => "the time-zone definition is invalid",
            Self::DuplicateZone => "the time-zone snapshot contains duplicate zones",
            Self::UnknownZone => "the time zone is not present in the snapshot",
            Self::OutOfRange => "the time is outside the time-zone snapshot's coverage",
            Self::AmbiguousLocalTime => "the local time occurs twice in this time zone",
            Self::NonexistentLocalTime => "the local time does not exist in this time zone",
        })
    }
}

impl std::error::Error for TimeZoneError {}

fn instant_cmp(left: OffsetDateTime, right: OffsetDateTime) -> std::cmp::Ordering {
    left.nanoseconds_since(right).cmp(&0)
}

/// Compares a zone-name segment such as `New_York` with a user-written
/// specifier such as `new york`, ignoring case, spaces, and underscores.
fn same_city(segment: &str, specifier: &str) -> bool {
    let normalized = |text: &str| {
        text.bytes()
            .filter(|byte| !matches!(byte, b' ' | b'_'))
            .map(|byte| byte.to_ascii_lowercase())
            .collect::<Vec<_>>()
    };
    normalized(segment) == normalized(specifier)
}
