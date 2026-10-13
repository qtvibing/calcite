//! Text-level normalization for conversational input.
//!
//! The strict parser is unchanged: these transforms only rewrite the source
//! text before it reaches the language. Callers try variants in order and use
//! the first that parses, so a normalization mistake can add an
//! interpretation but never change an existing correct one.

/// Leading conversational phrases. surface_variants removes them before
/// parsing.
const LEADING_PHRASES: &[&[&str]] = &[
    &["what", "is"],
    &["what's"],
    &["whats"],
    &["what", "are"],
    &["how", "much", "is"],
    &["how", "much"],
    &["how", "many"],
    &["tell", "me"],
    &["calculate"],
    &["compute"],
    &["evaluate"],
    &["convert"],
    &["calc"],
];

/// Returns the raw input followed by normalized variants worth trying.
///
/// The first entry is always the trimmed original. Callers that try the
/// variants in order get no behavior change for input the language already
/// accepts.
pub fn surface_variants(source: &str) -> Vec<String> {
    let raw = source.trim();
    let mut variants = vec![raw.to_owned()];
    let collapsed = collapse_whitespace(raw);
    let trimmed = collapsed.strip_suffix('?').unwrap_or(&collapsed);
    let normalized = strip_leading_phrases(trimmed);
    if normalized != raw {
        variants.push(normalized);
    }
    if let Some(rewritten) = rewrite_quantity_question(trimmed)
        && !variants.contains(&rewritten)
    {
        variants.push(rewritten);
    }
    variants
}

fn collapse_whitespace(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_leading_phrases(source: &str) -> String {
    let mut words: Vec<&str> = source.split(' ').collect();
    loop {
        let matched = LEADING_PHRASES
            .iter()
            .find(|phrase| {
                words.len() >= phrase.len()
                    && words
                        .iter()
                        .zip(phrase.iter())
                        .all(|(word, expected)| word.eq_ignore_ascii_case(expected))
            })
            .map(|phrase| phrase.len());
        match matched {
            Some(count) => {
                words.drain(..count);
            }
            None => break,
        }
    }
    words.join(" ")
}

/// Rewrites `how many X in Y` and `how much X in Y` questions as `Y in X`.
///
/// Runs on the collapsed input, without the question mark, before phrase
/// stripping. The quantity prefix is part of the pattern.
fn rewrite_quantity_question(source: &str) -> Option<String> {
    let words: Vec<&str> = source.split(' ').collect();
    let quantity = words.get(1)?;
    if !words.first()?.eq_ignore_ascii_case("how")
        || !(quantity.eq_ignore_ascii_case("many") || quantity.eq_ignore_ascii_case("much"))
    {
        return None;
    }
    let rest = &words[2..];
    let separator = rest
        .iter()
        .position(|word| word.eq_ignore_ascii_case("in"))?;
    let mut subject = &rest[..separator];
    let object = &rest[separator + 1..];
    // `are` is part of the quantity phrasing. `is` marks a statement.
    // The phrase stripping already handles statements.
    if subject
        .last()
        .is_some_and(|word| word.eq_ignore_ascii_case("are"))
    {
        subject = &subject[..subject.len() - 1];
    }
    if subject.is_empty()
        || object.is_empty()
        || subject.iter().any(|word| word.eq_ignore_ascii_case("is"))
    {
        return None;
    }
    Some(format!("{} in {}", object.join(" "), subject.join(" ")))
}
