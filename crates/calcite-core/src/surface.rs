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
    for rewritten in [
        rewrite_percent_question(trimmed),
        rewrite_directional_change(trimmed),
    ] {
        if let Some(rewritten) = rewritten
            && !variants.contains(&rewritten)
        {
            variants.push(rewritten);
        }
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

/// Rewrites `what % of 50 is 10` questions as `10 as a percentage of 50`.
fn rewrite_percent_question(source: &str) -> Option<String> {
    let words: Vec<&str> = source.split(' ').collect();
    let percent = words.get(1)?;
    let is_percent = *percent == "%"
        || percent.eq_ignore_ascii_case("percent")
        || percent.eq_ignore_ascii_case("percentage");
    if !words.first()?.eq_ignore_ascii_case("what")
        || !is_percent
        || !words.get(2)?.eq_ignore_ascii_case("of")
    {
        return None;
    }
    let separator = words[3..]
        .iter()
        .position(|word| word.eq_ignore_ascii_case("is"))?;
    let object = &words[3..separator + 3];
    let subject = &words[separator + 4..];
    if object.is_empty() || subject.is_empty() {
        return None;
    }
    Some(format!(
        "{} as a percentage of {}",
        subject.join(" "),
        object.join(" ")
    ))
}

fn change_sign(word: &str) -> Option<&'static str> {
    match word.to_ascii_lowercase().as_str() {
        "increase" | "increases" | "increased" | "raise" | "raised" | "grow" | "grew" | "grown" => {
            Some("+")
        }
        "decrease" | "decreases" | "decreased" | "reduce" | "reduced" | "lower" | "lowered" => {
            Some("-")
        }
        _ => None,
    }
}

/// Rewrites directional change phrasing as arithmetic: `increase 80 by
/// 15%` becomes `80 + 15%`, and `80 increased by 15%` becomes `80 + 15%`.
fn rewrite_directional_change(source: &str) -> Option<String> {
    let words: Vec<&str> = source.split(' ').collect();
    if let Some(sign) = words.first().copied().and_then(change_sign) {
        let separator = words
            .iter()
            .skip(1)
            .position(|word| word.eq_ignore_ascii_case("by"))?;
        let subject = &words[1..separator + 1];
        let change = &words[separator + 2..];
        if !subject.is_empty() && !change.is_empty() {
            return Some(format!(
                "{} {} {}",
                subject.join(" "),
                sign,
                change.join(" ")
            ));
        }
    }
    let mut rewritten: Vec<&str> = Vec::with_capacity(words.len());
    let mut changed = false;
    let mut index = 0;
    while index < words.len() {
        if let Some(sign) = words.get(index).copied().and_then(change_sign)
            && words
                .get(index + 1)
                .is_some_and(|word| word.eq_ignore_ascii_case("by"))
        {
            rewritten.push(sign);
            index += 2;
            changed = true;
        } else {
            rewritten.push(words[index]);
            index += 1;
        }
    }
    changed.then(|| rewritten.join(" "))
}
