use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pwshark", about = "NIST-compliant password generator", version)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Output raw password to stdout (no TUI)
    #[arg(long)]
    pub stdout: bool,

    /// Number of passwords to generate (stdout mode)
    #[arg(long, default_value_t = 1)]
    pub count: u16,

    /// Emit JSON (stdout mode): array of {password, entropy, strength}
    #[arg(long)]
    pub json: bool,

    /// Generation mode
    #[arg(long, value_enum, default_value = "random")]
    pub mode: ModeArg,

    /// Password length (random mode)
    #[arg(long, default_value_t = 16)]
    pub length: u8,

    /// Number of words (memorable mode)
    #[arg(long, default_value_t = 4)]
    pub words: u8,

    /// Word separator (memorable mode)
    #[arg(long, default_value = "-")]
    pub separator: String,

    /// Include uppercase letters (default: true)
    #[arg(long, overrides_with("no_uppercase"), default_value_t = true)]
    pub uppercase: bool,

    #[arg(long, hide = true)]
    pub no_uppercase: bool,

    /// Include lowercase letters (default: true)
    #[arg(long, overrides_with("no_lowercase"), default_value_t = true)]
    pub lowercase: bool,

    #[arg(long, hide = true)]
    pub no_lowercase: bool,

    /// Include numbers (default: true)
    #[arg(long, overrides_with("no_numbers"), default_value_t = true)]
    pub numbers: bool,

    #[arg(long, hide = true)]
    pub no_numbers: bool,

    /// Include symbols (default: true)
    #[arg(long, overrides_with("no_symbols"), default_value_t = true)]
    pub symbols: bool,

    #[arg(long, hide = true)]
    pub no_symbols: bool,

    /// Exclude visually ambiguous characters 0 O 1 l I (random mode)
    #[arg(long, overrides_with("no_exclude_ambiguous"), default_value_t = false)]
    pub exclude_ambiguous: bool,

    #[arg(long, hide = true)]
    pub no_exclude_ambiguous: bool,

    /// Random capitalization (memorable mode, default: true)
    #[arg(long, overrides_with("no_capitalize"), default_value_t = true)]
    pub capitalize: bool,

    #[arg(long, hide = true)]
    pub no_capitalize: bool,

    /// Add random numbers (memorable mode, default: true)
    #[arg(long, overrides_with("no_add_numbers"), default_value_t = true)]
    pub add_numbers: bool,

    #[arg(long, hide = true)]
    pub no_add_numbers: bool,

    /// Truncate words to ≤5 chars (memorable mode, default: true)
    #[arg(long, overrides_with("no_truncate"), default_value_t = true)]
    pub truncate: bool,

    #[arg(long, hide = true)]
    pub no_truncate: bool,
}

impl Args {
    pub fn get_uppercase(&self) -> bool {
        self.uppercase && !self.no_uppercase
    }
    pub fn get_lowercase(&self) -> bool {
        self.lowercase && !self.no_lowercase
    }
    pub fn get_numbers(&self) -> bool {
        self.numbers && !self.no_numbers
    }
    pub fn get_symbols(&self) -> bool {
        self.symbols && !self.no_symbols
    }
    pub fn get_exclude_ambiguous(&self) -> bool {
        self.exclude_ambiguous && !self.no_exclude_ambiguous
    }
    pub fn get_capitalize(&self) -> bool {
        self.capitalize && !self.no_capitalize
    }
    pub fn get_add_numbers(&self) -> bool {
        self.add_numbers && !self.no_add_numbers
    }
    pub fn get_truncate(&self) -> bool {
        self.truncate && !self.no_truncate
    }
}

#[derive(Subcommand, Clone)]
pub enum Command {
    /// Update pwshark to the latest version from Codeberg
    Update,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ModeArg {
    Random,
    Memorable,
}
