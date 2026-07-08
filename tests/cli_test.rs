use clap::Parser;
use pwshark::cli::{Args, Command};

#[test]
fn update_subcommand_parses() {
    let args = Args::try_parse_from(["pwshark", "update"]).unwrap();
    assert!(matches!(args.command, Some(Command::Update)));
}

#[test]
fn no_subcommand_still_parses_flags() {
    let args = Args::try_parse_from(["pwshark", "--stdout"]).unwrap();
    assert!(args.command.is_none());
    assert!(args.stdout);
}

#[test]
fn count_and_json_flags_parse() {
    let args = Args::try_parse_from(["pwshark", "--stdout", "--count", "5", "--json"]).unwrap();
    assert_eq!(args.count, 5);
    assert!(args.json);
}

#[test]
fn count_defaults_to_one() {
    let args = Args::try_parse_from(["pwshark", "--stdout"]).unwrap();
    assert_eq!(args.count, 1);
    assert!(!args.json);
}

#[test]
fn exclude_ambiguous_flag_parses() {
    let args = Args::try_parse_from(["pwshark", "--exclude-ambiguous"]).unwrap();
    assert!(args.get_exclude_ambiguous());
}

#[test]
fn notice_flag_parses() {
    let args = Args::try_parse_from(["pwshark", "--notice"]).unwrap();
    assert!(args.notice);
}

#[test]
fn notice_flag_prints_wordlist_attribution() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_pwshark"))
        .arg("--notice")
        .output()
        .expect("run pwshark --notice");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Orchard Street"), "missing wordlist name: {stdout}");
    assert!(stdout.contains("CC BY-SA 4.0"), "missing license: {stdout}");
}
