use clap::Parser;
use cli::{Args, Command, ModeArg};
use gen::{
    calculate_entropy, generate_memorable, generate_random, memorable_entropy, strength_label,
    MemorableConfig, RandomConfig,
};

pub mod cli;
pub mod gen;
pub mod tui;

// `pwshark update` re-runs the installer straight from the repo's main branch on
// GitHub, which resolves and downloads the latest tagged release binary.
const INSTALL_URL: &str = "https://raw.githubusercontent.com/greenseeing/pwshark/main/install.sh";

const MAX_COUNT: u16 = 1000;

// The embedded wordlist is CC BY-SA 4.0; its attribution must travel with the
// binary, since the documented install ships only the compiled binary.
const NOTICE: &str = include_str!("../NOTICE");

fn main() {
    let args = Args::parse();

    if args.notice {
        print!("{NOTICE}");
        return;
    }

    match args.command {
        Some(Command::Update) => run_update(),
        None => {
            if args.stdout {
                run_stdout(&args);
            } else {
                tui::run();
            }
        }
    }
}

fn run_update() {
    let status = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!("curl -fsSL {INSTALL_URL} | bash"))
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => std::process::exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("pwshark update: failed to launch installer: {e}");
            std::process::exit(1);
        }
    }
}

fn run_stdout(args: &Args) {
    let mut rng = rand::rng();
    let count = args.count.clamp(1, MAX_COUNT);

    let random_cfg = RandomConfig {
        length: args.length,
        uppercase: args.get_uppercase(),
        lowercase: args.get_lowercase(),
        numbers: args.get_numbers(),
        symbols: args.get_symbols(),
        exclude_ambiguous: args.get_exclude_ambiguous(),
    };
    let memorable_cfg = MemorableConfig {
        word_count: args.words,
        separator: args.separator.clone(),
        capitalize: args.get_capitalize(),
        add_numbers: args.get_add_numbers(),
        truncate: args.get_truncate(),
    };

    let mut json_items: Vec<String> = Vec::new();
    for _ in 0..count {
        let (password, entropy) = match args.mode {
            ModeArg::Random => {
                let pw = generate_random(&mut rng, &random_cfg);
                let e = calculate_entropy(pw.as_str());
                (pw, e)
            }
            ModeArg::Memorable => {
                let pw = generate_memorable(&mut rng, &memorable_cfg);
                let e = memorable_entropy(&memorable_cfg);
                (pw, e)
            }
        };
        if args.json {
            json_items.push(format!(
                "  {{\"password\": {}, \"entropy\": {:.1}, \"strength\": {}}}",
                json_string(password.as_str()),
                entropy,
                json_string(strength_label(entropy))
            ));
        } else {
            println!("{}", password.as_str());
        }
    }

    if args.json {
        println!("[\n{}\n]", json_items.join(",\n"));
    }
}

// Minimal JSON string encoder (avoids a serde dependency for one field).
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
