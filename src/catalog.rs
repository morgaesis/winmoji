use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use unicode_general_category::{GeneralCategory, get_general_category};

use crate::config::SkinTone;

const MAX_FUZZY_WORD_BYTES: usize = 48;
const MAX_QUERY_TOKENS: usize = 8;
const EMOTICONS: &[(&str, &str, &str)] = &[
    (":)", "Smiley", "happy face classic"),
    (":-)", "Smiley With Nose", "happy face classic"),
    (":D", "Big Grin", "happy laugh face"),
    (";)", "Wink", "happy playful face"),
    (";-)", "Wink With Nose", "happy playful face"),
    (":(", "Frown", "sad unhappy face"),
    (":'(", "Crying Face", "sad tears face"),
    (":P", "Tongue Out", "playful silly face"),
    (":-P", "Tongue Out With Nose", "playful silly face"),
    (":/", "Skeptical Face", "uncertain doubtful face"),
    (":|", "Neutral Face", "blank neutral face"),
    (":O", "Surprised Face", "shock surprise face"),
    ("XD", "Laughing Face", "laugh happy face"),
    ("^_^", "Happy Eyes", "happy cute face"),
    ("-_-", "Unamused Face", "tired annoyed face"),
    ("o_O", "Confused Face", "confused surprised face"),
    ("<3", "Heart", "love affection"),
    ("</3", "Broken Heart", "sad heartbreak"),
    ("¯\\_(ツ)_/¯", "Shrug", "shrug indifferent"),
    ("ಠ_ಠ", "Disapproval", "look disapprove annoyed"),
    ("ಥ_ಥ", "Crying", "cry sad tears"),
    ("ʕ•ᴥ•ʔ", "Bear", "cute bear animal"),
    ("(づ｡◕‿‿◕｡)づ", "Hug", "hug affection cute"),
    ("༼ つ ◕_◕ ༽つ", "Give", "give take energy"),
    ("(ง'̀-'́)ง", "Fight", "fight determined"),
    ("ヽ(•‿•)ノ", "Celebration", "happy celebrate"),
    ("(╯°□°)╯︵ ┻━┻", "Table Flip", "angry rage table"),
    ("┬─┬ ノ( ゜-゜ノ)", "Table Restore", "calm restore table"),
];

#[derive(Clone, Debug)]
pub struct Entry {
    pub glyph: String,
    pub name: String,
    pub keywords: String,
    pub kind: &'static str,
    pub emoji_group: Option<emojis::Group>,
    normalized_name: String,
}

#[derive(Clone, Copy, Debug)]
pub struct Match {
    pub index: usize,
    pub score: i32,
}

static CATALOG: OnceLock<Vec<Entry>> = OnceLock::new();

pub fn entries() -> &'static [Entry] {
    CATALOG.get_or_init(build_catalog)
}

/// Resolve the skin tone variant of a glyph. `None` means the glyph has no
/// variant for the requested tone (or the tone is the default), so the
/// original glyph applies.
pub fn toned(glyph: &str, tone: SkinTone) -> Option<&'static str> {
    let target = match tone {
        SkinTone::Default => return None,
        SkinTone::Light => emojis::SkinTone::Light,
        SkinTone::MediumLight => emojis::SkinTone::MediumLight,
        SkinTone::Medium => emojis::SkinTone::Medium,
        SkinTone::MediumDark => emojis::SkinTone::MediumDark,
        SkinTone::Dark => emojis::SkinTone::Dark,
    };
    let toned = emojis::get(glyph)?.with_skin_tone(target)?;
    (toned.as_str() != glyph).then(|| toned.as_str())
}

/// Whether a glyph has selectable skin tone variants.
pub fn supports_tones(glyph: &str) -> bool {
    emojis::get(glyph)
        .and_then(|emoji| emoji.skin_tones())
        .is_some_and(|mut tones| tones.nth(1).is_some())
}

/// How often the user has picked each glyph, keyed by the catalog glyph.
pub type UsageCounts = HashMap<String, u32>;

/// What a glyph the user has picked before is worth. Only entries that
/// already match the query compete, so this reorders answers rather than
/// introducing them, and it is deliberately large enough to cross a match
/// tier: something picked for this kind of query before is a better answer
/// than something whose name merely reads more literally.
const USAGE_BONUS_BASE: i32 = 500;
const USAGE_BONUS_STEP: i32 = 40;
const USAGE_BONUS_USES: u32 = 11;

fn usage_bonus(usage: &UsageCounts, glyph: &str) -> i32 {
    match usage.get(glyph).copied().unwrap_or(0) {
        0 => 0,
        uses => USAGE_BONUS_BASE + USAGE_BONUS_STEP * (uses.min(USAGE_BONUS_USES) - 1) as i32,
    }
}

pub fn search(query: &str, limit: usize, usage: &UsageCounts) -> Vec<Match> {
    if limit == 0 {
        return Vec::new();
    }

    let catalog = entries();
    let literal = query.trim().trim_end_matches('\u{fe0f}');
    if !literal.is_empty() {
        let mut literal_matches = catalog
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.glyph.trim_end_matches('\u{fe0f}') == literal)
            .map(|(index, _)| Match {
                index,
                score: 2_000,
            })
            .collect::<Vec<_>>();
        if !literal_matches.is_empty() {
            literal_matches.truncate(limit);
            return literal_matches;
        }
    }

    let query = normalize(query);
    if query.is_empty() {
        return catalog
            .iter()
            .enumerate()
            .take(limit)
            .map(|(index, _)| Match { index, score: 0 })
            .collect();
    }

    let query_tokens: Vec<_> = query.split_whitespace().take(MAX_QUERY_TOKENS).collect();
    let collect_matches = |allow_fuzzy| {
        catalog
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                score_entry(entry, &query_tokens, &query, allow_fuzzy)
                    .map(|score| score + usage_bonus(usage, &entry.glyph))
                    .map(|score| Match { index, score })
            })
            .collect::<Vec<_>>()
    };

    let mut matches = collect_matches(false);
    let strict_is_confident = matches.iter().any(|item| {
        let entry = &catalog[item.index];
        query_tokens.iter().all(|token| {
            entry.normalized_name == *token
                || entry
                    .normalized_name
                    .split_whitespace()
                    .any(|word| word == *token)
                || entry.keywords.split_whitespace().any(|word| word == *token)
                || curated_keywords(&entry.glyph)
                    .split_whitespace()
                    .any(|word| word == *token)
        })
    });
    if !strict_is_confident {
        matches = collect_matches(true);
    }

    let rank = |item: &Match| (Reverse(item.score), item.index);
    if matches.len() > limit {
        matches.select_nth_unstable_by_key(limit, rank);
        matches.truncate(limit);
    }
    matches.sort_unstable_by_key(rank);
    matches.truncate(limit);
    matches
}

fn score_entry(
    entry: &Entry,
    query_tokens: &[&str],
    query: &str,
    allow_fuzzy: bool,
) -> Option<i32> {
    let mut score = 0;
    for token in query_tokens {
        score += score_token(entry, token, allow_fuzzy)?;
    }

    if query_tokens.len() > 1 && entry.normalized_name == query {
        score += 260;
    }
    score += popularity_bonus(&entry.glyph);
    score -= entry.normalized_name.len().min(80) as i32;
    Some(score)
}

fn score_token(entry: &Entry, token: &str, allow_fuzzy: bool) -> Option<i32> {
    let name = &entry.normalized_name;
    let curated = curated_keywords(&entry.glyph);

    if name == token || curated.split_whitespace().any(|word| word == token) {
        return Some(1_400);
    }
    if name.split_whitespace().any(|word| word == token) {
        return Some(1_250);
    }
    if entry.keywords.split_whitespace().any(|word| word == token) {
        return Some(1_120);
    }
    if name.starts_with(token) {
        return Some(1_050);
    }
    if name.split_whitespace().any(|word| word.starts_with(token)) {
        return Some(950);
    }
    if curated
        .split_whitespace()
        .any(|word| word.starts_with(token))
    {
        return Some(925);
    }
    if entry
        .keywords
        .split_whitespace()
        .any(|word| word.starts_with(token))
    {
        return Some(850);
    }
    if !allow_fuzzy {
        return None;
    }
    if name.contains(token) || entry.keywords.contains(token) {
        return Some(700);
    }
    if let Some(score) = fuzzy_score(token, name, curated, &entry.keywords) {
        return Some(score);
    }
    if is_subsequence(token, name) || is_subsequence(token, &entry.keywords) {
        return Some(300);
    }
    None
}

fn fuzzy_score(token: &str, name: &str, curated: &str, keywords: &str) -> Option<i32> {
    let maximum_distance = fuzzy_distance_limit(token)?;
    let mut best = None;

    for (words, base_score) in [(curated, 950), (name, 920), (keywords, 840)] {
        let distance = words
            .split_whitespace()
            .filter_map(|word| bounded_edit_distance(token, word, maximum_distance))
            .min();
        if let Some(distance) = distance {
            let score = base_score - distance as i32 * 150;
            best = Some(best.map_or(score, |current: i32| current.max(score)));
        }
    }

    best
}

fn fuzzy_distance_limit(token: &str) -> Option<usize> {
    if !token.is_ascii() || !(4..=MAX_FUZZY_WORD_BYTES).contains(&token.len()) {
        return None;
    }

    Some(match token.len() {
        4..=5 => 1,
        6..=12 => 2,
        _ => 3,
    })
}

fn bounded_edit_distance(left: &str, right: &str, maximum: usize) -> Option<usize> {
    if !right.is_ascii()
        || left.len() > MAX_FUZZY_WORD_BYTES
        || right.len() > MAX_FUZZY_WORD_BYTES
        || left.len().abs_diff(right.len()) > maximum
    {
        return None;
    }
    if left == right {
        return Some(0);
    }

    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut previous_previous = [0_u8; MAX_FUZZY_WORD_BYTES + 1];
    let mut previous = [0_u8; MAX_FUZZY_WORD_BYTES + 1];
    let mut current = [0_u8; MAX_FUZZY_WORD_BYTES + 1];

    for (column, value) in previous.iter_mut().enumerate().take(right.len() + 1) {
        *value = column as u8;
    }

    for row in 1..=left.len() {
        current[0] = row as u8;
        for column in 1..=right.len() {
            let substitution_cost = u8::from(left[row - 1] != right[column - 1]);
            let mut distance = previous[column]
                .saturating_add(1)
                .min(current[column - 1].saturating_add(1))
                .min(previous[column - 1].saturating_add(substitution_cost));

            if row > 1
                && column > 1
                && left[row - 1] == right[column - 2]
                && left[row - 2] == right[column - 1]
            {
                distance = distance.min(previous_previous[column - 2].saturating_add(1));
            }
            current[column] = distance;
        }

        previous_previous = previous;
        previous = current;
    }

    let distance = previous[right.len()] as usize;
    (distance <= maximum).then_some(distance)
}

fn popularity_bonus(glyph: &str) -> i32 {
    match glyph {
        "😀" => 300,
        "😂" => 285,
        "🙂" => 270,
        "😊" => 255,
        "😍" => 245,
        "😉" => 235,
        "😅" => 225,
        "😁" => 215,
        "😄" => 205,
        "😃" => 195,
        "😆" => 185,
        "🤣" => 175,
        "😭" => 165,
        "😎" => 155,
        "❤️" | "❤" => 145,
        "👍" => 135,
        _ => 0,
    }
}

fn build_catalog() -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut seen = HashSet::new();

    for emoji in emojis::iter() {
        let glyph = emoji.as_str().to_string();
        seen.insert(glyph.clone());
        let mut keywords = emoji
            .shortcodes()
            .flat_map(|value| value.split('_'))
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>();
        keywords.extend(
            curated_keywords(emoji.as_str())
                .split_whitespace()
                .map(str::to_string),
        );
        keywords.push(normalize(&format!("{:?}", emoji.group())));
        let name = title_case(emoji.name());
        entries.push(Entry {
            glyph,
            normalized_name: normalize(&name),
            name,
            keywords: keywords.join(" "),
            kind: "Emoji",
            emoji_group: Some(emoji.group()),
        });
    }

    for codepoint in 0..=char::MAX as u32 {
        let Some(character) = char::from_u32(codepoint) else {
            continue;
        };
        if seen.contains(&character.to_string()) || !include_character(character) {
            continue;
        }
        let Some(name) = unicode_names2::name(character) else {
            continue;
        };
        let category = get_general_category(character);
        let name = title_case(&name.to_string());
        entries.push(Entry {
            glyph: character.to_string(),
            normalized_name: normalize(&name),
            name,
            keywords: curated_keywords(&character.to_string()).to_string(),
            kind: kind_for(category, character),
            emoji_group: None,
        });
    }

    for (glyph, name, keywords) in EMOTICONS {
        entries.push(Entry {
            glyph: (*glyph).to_string(),
            normalized_name: normalize(name),
            name: (*name).to_string(),
            keywords: (*keywords).to_string(),
            kind: "Emoticon",
            emoji_group: None,
        });
    }

    entries
}

fn include_character(character: char) -> bool {
    let codepoint = character as u32;
    let greek = (0x0370..=0x03ff).contains(&codepoint) || (0x1f00..=0x1fff).contains(&codepoint);
    let technical_ranges = matches!(codepoint, 0x00b2 | 0x00b3 | 0x00b9)
        || (0x2070..=0x209f).contains(&codepoint)
        || (0x2100..=0x214f).contains(&codepoint)
        || (0x1d400..=0x1d7ff).contains(&codepoint);
    let useful_punctuation = matches!(
        get_general_category(character),
        GeneralCategory::ConnectorPunctuation
            | GeneralCategory::DashPunctuation
            | GeneralCategory::OpenPunctuation
            | GeneralCategory::ClosePunctuation
            | GeneralCategory::InitialPunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::OtherPunctuation
    );
    greek
        || technical_ranges
        || useful_punctuation
        || matches!(
            get_general_category(character),
            GeneralCategory::MathSymbol
                | GeneralCategory::CurrencySymbol
                | GeneralCategory::ModifierSymbol
                | GeneralCategory::OtherSymbol
        )
}

fn kind_for(category: GeneralCategory, character: char) -> &'static str {
    let codepoint = character as u32;
    if (0x0370..=0x03ff).contains(&codepoint) || (0x1f00..=0x1fff).contains(&codepoint) {
        "Greek"
    } else {
        match category {
            GeneralCategory::MathSymbol => "Math",
            GeneralCategory::CurrencySymbol => "Currency",
            GeneralCategory::ConnectorPunctuation
            | GeneralCategory::DashPunctuation
            | GeneralCategory::OpenPunctuation
            | GeneralCategory::ClosePunctuation
            | GeneralCategory::InitialPunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::OtherPunctuation => "Punctuation",
            _ => "Symbol",
        }
    }
}

fn curated_keywords(glyph: &str) -> &'static str {
    match glyph {
        "😀" => "smile happy joy",
        "😂" => "laugh cry funny lol",
        "🙂" => "smile happy",
        "❤️" | "❤" => "heart love affection",
        "🔥" => "hot flame lit",
        "✅" | "✓" => "done success yes tick",
        "❌" | "✗" => "error fail no close",
        "⚠" | "⚠️" => "alert caution hazard warning",
        "👍" => "like approve yes",
        "👎" => "dislike reject no",
        "🙏" => "please thanks pray",
        "💡" => "idea insight",
        "🚀" => "launch ship fast",
        "🐛" => "defect debug",
        "🔒" => "secure private",
        "📎" => "attachment attach",
        "🔍" => "search find",
        "⚙" | "⚙️" => "settings configuration",
        "🧪" => "test experiment",
        "↻" | "⟳" => "refresh reload repeat",
        "↺" | "⟲" => "undo counterclockwise",
        "⇄" => "swap exchange",
        "←" => "left arrow leftwards",
        "→" => "right arrow rightwards",
        "↑" => "up arrow upwards",
        "↓" => "down arrow downwards",
        "⇒" => "implies implication",
        "⇔" => "equivalent iff",
        "λ" | "Λ" => "lambda greek",
        "∴" => "therefore conclusion",
        "∵" => "because reason",
        "ℝ" => "real numbers math",
        "ℂ" => "complex numbers math",
        "ℤ" => "integers numbers math",
        "ℕ" => "natural numbers math",
        "⌘" => "command key keyboard",
        "⌥" => "option alt keyboard",
        "⌫" => "backspace delete keyboard",
        "⏎" => "return enter newline keyboard",
        "⎋" => "escape esc keyboard",
        "␣" => "space keyboard",
        "…" => "ellipsis dots horizontal",
        _ => "",
    }
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn title_case(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut beginning = true;
    for character in value.chars() {
        if beginning {
            result.extend(character.to_uppercase());
            beginning = false;
        } else {
            result.extend(character.to_lowercase());
        }
        if character == ' ' || character == '-' {
            beginning = true;
        }
    }
    result
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut characters = needle.chars();
    let mut wanted = characters.next();
    for character in haystack.chars() {
        if Some(character) == wanted {
            wanted = characters.next();
            if wanted.is_none() {
                return true;
            }
        }
    }
    wanted.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top(query: &str) -> String {
        let result = search(query, 8, &UsageCounts::new())
            .first()
            .copied()
            .unwrap_or_else(|| panic!("expected a result for {query}"));
        entries()[result.index].glyph.clone()
    }

    #[test]
    fn builds_full_offline_catalog() {
        assert!(
            entries().len() > 4_000,
            "catalog has {} entries",
            entries().len()
        );
    }

    #[test]
    fn finds_representative_unicode_families() {
        assert_eq!(top("rocket"), "🚀");
        assert_eq!(top("rightwards arrow"), "→");
        assert_eq!(top("integral"), "∫");
        assert_eq!(top("union"), "∪");
        assert_eq!(top("euro"), "€");
        assert_eq!(top("lambda"), "λ");
        assert!(["©", "©️"].contains(&top("copyright").as_str()));
        assert_eq!(top("ellipsis"), "…");
    }

    #[test]
    fn finds_entries_outside_curated_overrides() {
        assert_eq!(top("pretzel"), "🥨");
        assert_eq!(top("otter"), "🦦");
        assert!(["☃", "☃️"].contains(&top("snowman").as_str()));
        assert_eq!(top("perpendicular"), "⟂");
        assert_eq!(top("real numbers"), "ℝ");
        assert_eq!(top("complex numbers"), "ℂ");
        assert_eq!(top("superscript two"), "²");
        assert_eq!(top("mathematical bold capital a"), "𝐀");
    }

    #[test]
    fn ranks_curated_keywords() {
        assert_eq!(top("warn"), "⚠️");
        assert!(["↻", "⟳"].contains(&top("reload").as_str()));
        let done = top("done");
        assert!(["✅", "✓"].contains(&done.as_str()), "found {done}");
    }

    #[test]
    fn ranks_familiar_emoji_ahead_of_obscure_symbols() {
        assert_eq!(top("smile"), "😀");
        assert_eq!(top("grin"), "😀");
    }

    /// The plain emoji for a word outranks both the compound names that
    /// contain it (Smiling Face With Heart-Eyes) and the emoticon whose name
    /// happens to be exactly that word.
    #[test]
    fn ranks_the_plain_emoji_ahead_of_names_that_merely_contain_it() {
        assert_eq!(top("heart"), "❤️");
    }

    /// Picking an entry teaches the ranking. "frown" leads with the symbol
    /// literally named Frown until the user picks the frowning face, after
    /// which their choice leads and the symbol falls behind it.
    #[test]
    fn picking_an_entry_lifts_it_above_a_more_literal_name() {
        assert_eq!(top("frown"), "⌢");

        let mut usage = UsageCounts::new();
        usage.insert("☹️".to_string(), 1);
        let ranked = search("frown", 8, &usage);
        let glyph = |position: usize| entries()[ranked[position].index].glyph.as_str();
        assert_eq!(glyph(0), "☹️");
        assert!(
            ranked
                .iter()
                .any(|found| entries()[found.index].glyph == "⌢"),
            "the literal name is deranked, not dropped"
        );
    }

    #[test]
    fn usage_weight_saturates_rather_than_growing_without_bound() {
        let bonus = |uses| {
            let mut usage = UsageCounts::new();
            usage.insert("🚀".to_string(), uses);
            usage_bonus(&usage, "🚀")
        };
        assert_eq!(bonus(0), 0);
        assert!(bonus(1) < bonus(2));
        assert_eq!(bonus(USAGE_BONUS_USES), bonus(USAGE_BONUS_USES + 90));
    }

    #[test]
    fn tolerates_common_typing_errors() {
        assert_eq!(top("smiel"), "😀");
        assert_eq!(top("grining"), "😀");
        assert_eq!(top("perpendiculr"), "⟂");
        assert_eq!(top("right arorw"), "→");
        assert_eq!(top("uniond"), "∪");
    }

    #[test]
    fn ignores_search_punctuation() {
        let results = search("'smi", 8, &UsageCounts::new());
        assert!(!results.is_empty());
        assert!(
            results
                .iter()
                .any(|result| entries()[result.index].normalized_name.contains("smil"))
        );
        assert_eq!(top("\"smile\""), "😀");
        assert_eq!(top("right-arrow"), "→");
    }

    #[test]
    fn finds_literal_glyphs() {
        assert_eq!(top("∪"), "∪");
        assert_eq!(top("⟂"), "⟂");
        assert_eq!(top("😀"), "😀");
    }

    #[test]
    fn bounded_distance_handles_edits_and_transpositions() {
        assert_eq!(bounded_edit_distance("smiel", "smile", 1), Some(1));
        assert_eq!(bounded_edit_distance("grining", "grinning", 2), Some(1));
        assert_eq!(bounded_edit_distance("emoji", "emojis", 1), Some(1));
        assert_eq!(bounded_edit_distance("smile", "symbol", 2), None);
    }

    #[test]
    fn caps_result_count() {
        assert_eq!(search("arrow", 8, &UsageCounts::new()).len(), 8);
        assert!(search("arrow", 0, &UsageCounts::new()).is_empty());
    }

    #[test]
    fn resolves_skin_tone_variants() {
        assert_eq!(toned("👋", SkinTone::Medium), Some("👋🏽"));
        assert_eq!(toned("👋", SkinTone::Default), None);
        assert_eq!(toned("😀", SkinTone::Dark), None);
        assert!(supports_tones("👋"));
        assert!(!supports_tones("😀"));
        assert!(!supports_tones("→"));
    }
}
