use rand::Rng;
use std::sync::OnceLock;
use zeroize::Zeroize;

const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const NUMBERS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*()_+-=,.<>?";

// Visually confusable glyphs dropped when `exclude_ambiguous` is set.
const AMBIGUOUS: &str = "0O1lI";

pub const WORD_LIST: &str = include_str!("../wordlist.txt");

#[derive(Clone, Debug)]
pub enum Mode {
    Random,
    Memorable,
}

#[derive(Clone, Debug)]
pub struct RandomConfig {
    pub length: u8,
    pub uppercase: bool,
    pub lowercase: bool,
    pub numbers: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
}

impl Default for RandomConfig {
    fn default() -> Self {
        Self {
            length: 16,
            uppercase: true,
            lowercase: true,
            numbers: true,
            symbols: true,
            exclude_ambiguous: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MemorableConfig {
    pub word_count: u8,
    pub separator: String,
    pub capitalize: bool,
    pub add_numbers: bool,
    pub truncate: bool,
}

impl Default for MemorableConfig {
    fn default() -> Self {
        Self {
            word_count: 4,
            separator: "-".into(),
            capitalize: true,
            add_numbers: true,
            truncate: true,
        }
    }
}

#[derive(Zeroize)]
#[zeroize(drop)]
pub struct Password(String);

impl Password {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn next_u32_bounded(rng: &mut impl Rng, bound: u32) -> u32 {
    rng.random_range(0..bound)
}

pub fn generate_random(rng: &mut impl Rng, cfg: &RandomConfig) -> Password {
    let mut charset = String::new();
    if cfg.uppercase {
        charset.push_str(UPPERCASE);
    }
    if cfg.lowercase {
        charset.push_str(LOWERCASE);
    }
    if cfg.numbers {
        charset.push_str(NUMBERS);
    }
    if cfg.symbols {
        charset.push_str(SYMBOLS);
    }
    if charset.is_empty() {
        charset.push_str(LOWERCASE);
    }
    if cfg.exclude_ambiguous {
        charset.retain(|c| !AMBIGUOUS.contains(c));
    }
    let chars: Vec<char> = charset.chars().collect();
    let len = cfg.length.clamp(8, 64) as usize;
    let password: String = (0..len)
        .map(|_| {
            let idx = next_u32_bounded(rng, chars.len() as u32) as usize;
            chars[idx]
        })
        .collect();
    Password(password)
}

pub fn generate_memorable(rng: &mut impl Rng, cfg: &MemorableConfig) -> Password {
    let words_raw: Vec<&str> = WORD_LIST.lines().collect();
    if words_raw.is_empty() {
        return Password("error-empty-wordlist".into());
    }
    let count = cfg.word_count.clamp(3, 8) as usize;
    let indices: Vec<usize> = (0..count)
        .map(|_| next_u32_bounded(rng, words_raw.len() as u32) as usize)
        .collect();
    let mut words: Vec<String> = indices.iter().map(|&i| words_raw[i].to_string()).collect();

    if cfg.truncate {
        for word in &mut words {
            *word = truncate_word(word);
        }
    }

    let mut password = words.join(&cfg.separator);

    if cfg.capitalize {
        password = apply_random_capitalization(rng, &password);
    }

    if cfg.add_numbers {
        password = apply_random_numbers(rng, &password);
    }

    Password(password)
}

pub fn truncate_word(word: &str) -> String {
    const MAX: usize = 5;
    let chars: Vec<char> = word.chars().collect();
    if chars.len() <= MAX {
        return word.to_string();
    }
    let vowels = "aeiouAEIOU";
    let mut result = String::new();
    let mut first_vowel = false;
    for ch in chars {
        if result.len() >= MAX {
            break;
        }
        if vowels.contains(ch) {
            if !first_vowel {
                result.push(ch);
                first_vowel = true;
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn apply_random_capitalization(rng: &mut impl Rng, password: &str) -> String {
    let letter_positions: Vec<usize> = password
        .chars()
        .enumerate()
        .filter(|(_, c)| c.is_ascii_alphabetic())
        .map(|(i, _)| i)
        .collect();
    if letter_positions.is_empty() {
        return password.to_string();
    }
    let num_caps = (next_u32_bounded(rng, 3) + 1).min(letter_positions.len() as u32) as usize;
    let mut chars: Vec<char> = password.chars().collect();
    let mut selected = std::collections::HashSet::new();
    for _ in 0..num_caps {
        let pos = letter_positions[next_u32_bounded(rng, letter_positions.len() as u32) as usize];
        selected.insert(pos);
    }
    for pos in selected {
        let mut char_iter = password.chars();
        let char_at_pos = char_iter.nth(pos).unwrap();
        if let Some(c) = chars.get_mut(pos) {
            *c = char_at_pos.to_ascii_uppercase();
        }
    }
    chars.into_iter().collect()
}

fn apply_random_numbers(rng: &mut impl Rng, password: &str) -> String {
    let count = (next_u32_bounded(rng, 3) + 1) as usize;
    let mut chars: Vec<char> = password.chars().collect();
    for _ in 0..count {
        let digit = char::from_digit(next_u32_bounded(rng, 10), 10).unwrap();
        let pos = next_u32_bounded(rng, chars.len() as u32 + 1) as usize;
        chars.insert(pos, digit);
    }
    chars.into_iter().collect()
}

// Number of distinct words the memorable generator can actually draw. Truncation
// (truncate_word) collapses near-duplicates ("abdomen"/"abdominal" -> "abdmn"), so
// the effective alphabet is smaller than the raw line count and entropy with it.
pub fn effective_word_pool(truncate: bool) -> usize {
    static FULL: OnceLock<usize> = OnceLock::new();
    static TRUNCATED: OnceLock<usize> = OnceLock::new();
    if truncate {
        *TRUNCATED.get_or_init(|| {
            WORD_LIST
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(truncate_word)
                .collect::<std::collections::HashSet<_>>()
                .len()
        })
    } else {
        *FULL.get_or_init(|| WORD_LIST.lines().filter(|l| !l.trim().is_empty()).count())
    }
}

// Entropy of a diceware passphrase. The charset heuristic in `calculate_entropy`
// is meaningless here (words are drawn from a finite list, not per-character), so
// we count the real choices: word_count * log2(pool), plus a deliberately
// conservative bonus for the random capitalization / digit insertion — those add
// little next to another word and we under-credit them on purpose.
pub fn memorable_entropy(cfg: &MemorableConfig) -> f64 {
    let pool = effective_word_pool(cfg.truncate);
    if pool == 0 {
        return 0.0;
    }
    let count = cfg.word_count.clamp(3, 8) as f64;
    let mut bits = count * (pool as f64).log2();
    if cfg.capitalize {
        // 1 of {1,2,3} positions capitalized.
        bits += 3.0_f64.log2();
    }
    if cfg.add_numbers {
        // 1 of {1,2,3} digits, each carrying at least its 0-9 value.
        bits += 3.0_f64.log2() + 10.0_f64.log2();
    }
    bits
}

// Charset-based entropy for RANDOM mode only (each position is an independent
// uniform draw from the pool). Do NOT use for memorable mode — see memorable_entropy.
pub fn calculate_entropy(password: &str) -> f64 {
    if password.is_empty() {
        return 0.0;
    }
    let mut charset_size = 0u64;
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_symbol = password.chars().any(|c| !c.is_ascii_alphanumeric());
    if has_lower {
        charset_size += 26;
    }
    if has_upper {
        charset_size += 26;
    }
    if has_digit {
        charset_size += 10;
    }
    if has_symbol {
        charset_size += SYMBOLS.len() as u64;
    }
    if charset_size == 0 {
        return 0.0;
    }
    (charset_size as f64).log2() * password.len() as f64
}

pub fn strength_label(entropy: f64) -> &'static str {
    if entropy < 40.0 {
        "Weak"
    } else if entropy < 60.0 {
        "Fair"
    } else if entropy < 80.0 {
        "Good"
    } else {
        "Strong"
    }
}

pub fn separator_presets() -> Vec<&'static str> {
    vec!["-", ".", "_", "/", " ", ""]
}
