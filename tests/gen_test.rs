use pwshark::gen::{calculate_entropy, effective_word_pool, generate_memorable, generate_random,
    memorable_entropy, strength_label, MemorableConfig, RandomConfig};

fn make_rng() -> impl rand::Rng {
    rand::rng()
}

#[test]
fn random_password_has_requested_length() {
    let mut rng = make_rng();
    let cfg = RandomConfig {
        length: 20,
        uppercase: true,
        lowercase: true,
        numbers: true,
        symbols: true,
        exclude_ambiguous: false,
    };
    let pw = generate_random(&mut rng, &cfg);
    assert_eq!(pw.as_str().len(), 20);
}

#[test]
fn random_password_uses_only_enabled_charsets() {
    let mut rng = make_rng();
    let cfg = RandomConfig {
        length: 64,
        uppercase: true,
        lowercase: false,
        numbers: false,
        symbols: false,
        exclude_ambiguous: false,
    };
    let pw = generate_random(&mut rng, &cfg);
    for c in pw.as_str().chars() {
        assert!(c.is_ascii_uppercase(), "found non-uppercase: {c}");
    }
}

#[test]
fn random_password_respects_min_8_max_64() {
    let mut rng = make_rng();
    let cfg_too_short = RandomConfig { length: 3, ..Default::default() };
    let pw = generate_random(&mut rng, &cfg_too_short);
    assert!(pw.as_str().len() >= 8);

    let cfg_too_long = RandomConfig { length: 99, ..Default::default() };
    let pw = generate_random(&mut rng, &cfg_too_long);
    assert!(pw.as_str().len() <= 64);
}

#[test]
fn memorable_password_has_correct_word_count() {
    let mut rng = make_rng();
    let cfg = MemorableConfig {
        word_count: 4,
        separator: "-".into(),
        capitalize: false,
        add_numbers: false,
        truncate: false,
    };
    let pw = generate_memorable(&mut rng, &cfg);
    let parts: Vec<&str> = pw.as_str().split('-').collect();
    assert_eq!(parts.len(), 4);
}

#[test]
fn memorable_truncation_limits_word_length() {
    let mut rng = make_rng();
    let cfg = MemorableConfig {
        word_count: 8,
        separator: "-".into(),
        capitalize: false,
        add_numbers: false,
        truncate: true,
    };
    let pw = generate_memorable(&mut rng, &cfg);
    for word in pw.as_str().split('-') {
        assert!(word.len() <= 5, "word '{word}' exceeds 5 chars");
    }
}

#[test]
fn entropy_increases_with_length() {
    let mut rng = make_rng();
    let short = generate_random(&mut rng, &RandomConfig { length: 8, ..Default::default() });
    let long = generate_random(&mut rng, &RandomConfig { length: 32, ..Default::default() });
    let e_short = calculate_entropy(short.as_str());
    let e_long = calculate_entropy(long.as_str());
    assert!(e_long > e_short, "32-char should have more entropy than 8-char");
}

#[test]
fn strength_labels_match_entropy() {
    assert_eq!(strength_label(0.0), "Weak");
    assert_eq!(strength_label(39.9), "Weak");
    assert_eq!(strength_label(40.0), "Fair");
    assert_eq!(strength_label(59.9), "Fair");
    assert_eq!(strength_label(60.0), "Good");
    assert_eq!(strength_label(79.9), "Good");
    assert_eq!(strength_label(80.0), "Strong");
}

#[test]
fn password_text_survives_to_string_copy() {
    let mut rng = make_rng();
    let pw = generate_random(&mut rng, &RandomConfig::default());
    let original = pw.as_str().to_string();
    assert!(!original.is_empty());
    // The string copy should survive even after Password is dropped
    let copy = original.clone();
    drop(pw);
    assert_eq!(original, copy);
}

#[test]
fn truncate_word_keeps_first_vowel_and_consonants() {
    use pwshark::gen::truncate_word;
    assert_eq!(truncate_word("cat"), "cat");
    assert_eq!(truncate_word("seemingly"), "semng");
    assert_eq!(truncate_word("abdominal"), "abdmn");
    assert_eq!(truncate_word("enrage"), "enrg");
}

#[test]
fn effective_word_pool_matches_wordlist() {
    // EFF diceware list has 7776 entries; truncation collapses near-duplicates.
    assert_eq!(effective_word_pool(false), 7776);
    let truncated = effective_word_pool(true);
    assert!(truncated < 7776, "truncation must reduce the pool");
    assert!(truncated > 6000, "but not collapse it drastically: {truncated}");
}

#[test]
fn memorable_entropy_is_realistic_not_charset_inflated() {
    // The old charset heuristic reported ~140 bits ("Strong") for default 4 words.
    // Real diceware entropy is ~50-65 bits, so it must land below the Strong cutoff.
    let cfg = MemorableConfig::default();
    let e = memorable_entropy(&cfg);
    assert!(e > 40.0, "4-word passphrase should clear Weak: {e}");
    assert!(e < 80.0, "4-word passphrase must not read Strong: {e}");
    assert_ne!(strength_label(e), "Strong");
}

#[test]
fn memorable_three_words_not_labeled_strong() {
    let cfg = MemorableConfig { word_count: 3, ..Default::default() };
    let e = memorable_entropy(&cfg);
    assert!(e < 80.0, "3-word passphrase must not read Strong: {e}");
    assert_ne!(strength_label(e), "Strong");
}

#[test]
fn memorable_entropy_grows_with_word_count() {
    let three = memorable_entropy(&MemorableConfig { word_count: 3, ..Default::default() });
    let seven = memorable_entropy(&MemorableConfig { word_count: 7, ..Default::default() });
    assert!(seven > three, "more words must mean more entropy");
}

#[test]
fn exclude_ambiguous_removes_confusable_chars() {
    let mut rng = make_rng();
    let cfg = RandomConfig {
        length: 64,
        uppercase: true,
        lowercase: true,
        numbers: true,
        symbols: true,
        exclude_ambiguous: true,
    };
    let pw = generate_random(&mut rng, &cfg);
    for c in pw.as_str().chars() {
        assert!(!"0O1lI".contains(c), "ambiguous char leaked: {c}");
    }
}

#[test]
fn truncate_word_handles_char_boundary() {
    use pwshark::gen::truncate_word;
    // Unicode word where .len() != .chars().count()
    let cafe = "caf\u{00e9}"; // "café" — 4 chars, 5 bytes
    let truncated = truncate_word(cafe);
    assert!(truncated.chars().count() <= 5);
}
