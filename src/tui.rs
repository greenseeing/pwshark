use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::time::Duration;

use crate::gen::{
    calculate_entropy, generate_memorable, generate_random, memorable_entropy, separator_presets,
    strength_label, MemorableConfig, Mode, Password, RandomConfig,
};

struct App {
    mode: Mode,
    random_cfg: RandomConfig,
    memorable_cfg: MemorableConfig,
    password: Option<Password>,
    entropy: f64,
    focus: Focus,
    separator_idx: usize,
    separator_custom: Option<String>,
    editing_separator: bool,
    clipboard_msg: Option<String>,
    clipboard: Option<arboard::Clipboard>,
}

enum Focus {
    ModeSelector,
    RandomOpt(usize),
    MemorableOpt(usize),
}

impl App {
    fn new() -> Self {
        Self {
            mode: Mode::Random,
            random_cfg: RandomConfig::default(),
            memorable_cfg: MemorableConfig::default(),
            password: None,
            entropy: 0.0,
            focus: Focus::ModeSelector,
            separator_idx: 0,
            separator_custom: None,
            editing_separator: false,
            clipboard_msg: None,
            clipboard: None,
        }
    }

    fn generate(&mut self) {
        let mut rng = rand::rng();
        let (pw, entropy) = match self.mode {
            Mode::Random => {
                let pw = generate_random(&mut rng, &self.random_cfg);
                let e = calculate_entropy(pw.as_str());
                (pw, e)
            }
            Mode::Memorable => {
                let sep = self.current_separator();
                let pw = generate_memorable(
                    &mut rng,
                    &MemorableConfig {
                        separator: sep,
                        ..self.memorable_cfg.clone()
                    },
                );
                // Diceware entropy is set by word selection, not the rendered charset.
                let e = memorable_entropy(&self.memorable_cfg);
                (pw, e)
            }
        };
        self.entropy = entropy;
        self.password = Some(pw);
    }

    fn copy_to_clipboard(&mut self) {
        if let Some(ref pw) = self.password {
            let text = pw.as_str().to_string();
            if self.clipboard.is_none() {
                self.clipboard = arboard::Clipboard::new().ok();
            }
            if let Some(ref mut cb) = self.clipboard {
                match cb.set_text(&text) {
                    Ok(()) => {
                        self.clipboard_msg = Some("Copied! Clears in 15s".into());
                    }
                    Err(e) => {
                        self.clipboard_msg = Some(format!("Copy failed: {e}"));
                    }
                }
            } else {
                self.clipboard_msg = Some("Clipboard unavailable".into());
            }
        }
    }

    fn current_separator(&self) -> String {
        self.separator_custom
            .clone()
            .unwrap_or_else(|| separator_presets()[self.separator_idx].to_string())
    }

    fn random_option_count(&self) -> usize {
        6
    }

    fn memorable_option_count(&self) -> usize {
        5
    }
}

pub fn run() {
    enable_raw_mode().expect("raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).expect("alt screen");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let mut app = App::new();
    app.generate();

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode().expect("disable raw");
    execute!(terminal.backend_mut(), LeaveAlternateScreen).expect("leave alt");
    terminal.show_cursor().expect("cursor");
    if let Err(e) = res {
        eprintln!("Error: {e}");
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let mut clipboard_clear_timer: Option<std::time::Instant> = None;

    loop {
        terminal.draw(|f| draw(f, app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if app.editing_separator {
                    handle_separator_edit(key.code, app);
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('y') => {
                        app.copy_to_clipboard();
                        clipboard_clear_timer = Some(std::time::Instant::now());
                    }
                    KeyCode::Char(c) => {
                        if matches!(app.focus, Focus::MemorableOpt(1)) {
                            app.editing_separator = true;
                            app.separator_custom = Some(c.to_string());
                        }
                    }
                    KeyCode::Enter => app.generate(),
                    KeyCode::Tab => {
                        app.mode = match app.mode {
                            Mode::Random => Mode::Memorable,
                            Mode::Memorable => Mode::Random,
                        };
                        app.focus = match app.mode {
                            Mode::Random => Focus::RandomOpt(0),
                            Mode::Memorable => Focus::MemorableOpt(0),
                        };
                        app.generate();
                    }
                    KeyCode::Up => move_focus(app, -1),
                    KeyCode::Down => move_focus(app, 1),
                    KeyCode::Left => adjust_option(app, -1),
                    KeyCode::Right => adjust_option(app, 1),
                    _ => {}
                }
            }
        }

        if let Some(timer) = clipboard_clear_timer {
            if timer.elapsed() >= Duration::from_secs(15) {
                if let Some(ref mut cb) = app.clipboard {
                    let _ = cb.set_text("");
                }
                app.clipboard_msg = None;
                clipboard_clear_timer = None;
            }
        }
    }
}

fn handle_separator_edit(code: KeyCode, app: &mut App) {
    match code {
        KeyCode::Esc | KeyCode::Enter => {
            app.editing_separator = false;
            app.generate();
        }
        KeyCode::Backspace => {
            if let Some(ref mut s) = app.separator_custom {
                s.pop();
                if s.is_empty() {
                    app.separator_custom = None;
                }
            }
        }
        KeyCode::Char(c) => {
            if app.separator_custom.is_none() {
                app.separator_custom = Some(String::new());
            }
            if let Some(ref mut s) = app.separator_custom {
                if s.len() < 3 {
                    s.push(c);
                }
            }
        }
        _ => {}
    }
}

fn move_focus(app: &mut App, delta: i8) {
    match &app.focus {
        Focus::ModeSelector => {
            if delta > 0 {
                app.focus = match app.mode {
                    Mode::Random => Focus::RandomOpt(0),
                    Mode::Memorable => Focus::MemorableOpt(0),
                };
            }
        }
        Focus::RandomOpt(i) => {
            let max = app.random_option_count();
            let new = (*i as i8 + delta).clamp(0, (max - 1) as i8) as usize;
            if new == 0 && delta < 0 && *i == 0 {
                app.focus = Focus::ModeSelector;
            } else {
                app.focus = Focus::RandomOpt(new);
            }
        }
        Focus::MemorableOpt(i) => {
            let max = app.memorable_option_count();
            let new = (*i as i8 + delta).clamp(0, (max - 1) as i8) as usize;
            if new == 0 && delta < 0 && *i == 0 {
                app.focus = Focus::ModeSelector;
            } else {
                app.focus = Focus::MemorableOpt(new);
            }
        }
    }
}

fn adjust_option(app: &mut App, delta: i8) {
    match &app.focus {
        Focus::ModeSelector => {
            if delta > 0 {
                app.mode = Mode::Memorable;
                app.focus = Focus::MemorableOpt(0);
            } else {
                app.mode = Mode::Random;
                app.focus = Focus::RandomOpt(0);
            }
            app.generate();
        }
        Focus::RandomOpt(i) => match *i {
            0 => {
                let len = &mut app.random_cfg.length;
                *len = ((*len as i8 + delta).clamp(8, 64)) as u8;
                app.generate();
            }
            1 => {
                app.random_cfg.uppercase = !app.random_cfg.uppercase;
                app.generate();
            }
            2 => {
                app.random_cfg.lowercase = !app.random_cfg.lowercase;
                app.generate();
            }
            3 => {
                app.random_cfg.numbers = !app.random_cfg.numbers;
                app.generate();
            }
            4 => {
                app.random_cfg.symbols = !app.random_cfg.symbols;
                app.generate();
            }
            5 => {
                app.random_cfg.exclude_ambiguous = !app.random_cfg.exclude_ambiguous;
                app.generate();
            }
            _ => {}
        },
        Focus::MemorableOpt(i) => match *i {
            0 => {
                let wc = &mut app.memorable_cfg.word_count;
                *wc = ((*wc as i8 + delta).clamp(3, 8)) as u8;
                app.generate();
            }
            1 => {
                if app.separator_custom.is_none() && !app.editing_separator {
                    let presets = separator_presets();
                    let new = if delta > 0 {
                        (app.separator_idx + 1) % presets.len()
                    } else {
                        (app.separator_idx + presets.len() - 1) % presets.len()
                    };
                    app.separator_idx = new;
                } else {
                    app.separator_custom = None;
                }
                app.generate();
            }
            2 => {
                app.memorable_cfg.capitalize = !app.memorable_cfg.capitalize;
                app.generate();
            }
            3 => {
                app.memorable_cfg.add_numbers = !app.memorable_cfg.add_numbers;
                app.generate();
            }
            4 => {
                app.memorable_cfg.truncate = !app.memorable_cfg.truncate;
                app.generate();
            }
            _ => {}
        },
    }
}

fn draw(f: &mut Frame, app: &App) {
    let size = f.area();
    let two_column = size.width >= 80;

    let (left_area, right_area) = if two_column {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(size);
        (chunks[0], chunks[1])
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(size);
        (chunks[0], chunks[1])
    };

    draw_left(f, app, left_area);
    draw_right(f, app, right_area);

    // Keybinding hints at the bottom of the screen
    draw_hints(f, size);
}

fn draw_left(f: &mut Frame, app: &App, area: Rect) {
    let mode_focused = matches!(app.focus, Focus::ModeSelector);

    let mode_line = Line::from(vec![
        Span::styled(
            if mode_focused { "> " } else { "  " },
            Style::default().fg(GREEN),
        ),
        Span::styled(
            "[Random]",
            Style::default()
                .fg(if matches!(app.mode, Mode::Random) { GREEN } else { DIM })
                .add_modifier(if mode_focused && matches!(app.mode, Mode::Random) {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
        Span::raw("  "),
        Span::styled(
            "[Memorable]",
            Style::default()
                .fg(if matches!(app.mode, Mode::Memorable) { GREEN } else { DIM })
                .add_modifier(if mode_focused && matches!(app.mode, Mode::Memorable) {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
    ]);

    let options_lines = match app.mode {
        Mode::Random => draw_random_options(app),
        Mode::Memorable => draw_memorable_options(app),
    };

    let mode_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(
            " pwshark ",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ));

    let mut lines: Vec<Line> = vec![mode_line, Line::raw("")];
    lines.extend(options_lines);

    let para = Paragraph::new(lines).block(mode_block).wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

fn draw_right(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(5)])
        .split(area);

    draw_password_panel(f, app, chunks[0]);
    draw_strength_panel(f, app, chunks[1]);
}

fn draw_password_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(
            " Password ",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    if let Some(ref pw) = app.password {
        let spans: Vec<Span> = pw
            .as_str()
            .chars()
            .map(|c| match c {
                c if c.is_ascii_uppercase() => {
                    Span::styled(c.to_string(), Style::default().fg(TEXT).add_modifier(Modifier::BOLD))
                }
                c if c.is_ascii_lowercase() => {
                    Span::styled(c.to_string(), Style::default().fg(DIM))
                }
                c if c.is_ascii_digit() => {
                    Span::styled(c.to_string(), Style::default().fg(ORANGE))
                }
                _ => {
                    Span::styled(c.to_string(), Style::default().fg(BLUE))
                }
            })
            .collect();

        lines.push(Line::raw(""));
        lines.push(Line::from(spans));
        lines.push(Line::raw(""));

        if let Some(ref msg) = app.clipboard_msg {
            lines.push(Line::from(Span::styled(
                format!("  {msg}"),
                Style::default().fg(GREEN),
            )));
        }

        let pw_len = pw.as_str().len();
        lines.push(Line::from(Span::styled(
            format!("  {} chars", pw_len),
            Style::default().fg(DIM),
        )));
    }

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

fn draw_strength_panel(f: &mut Frame, app: &App, area: Rect) {
    let label = strength_label(app.entropy);
    let color = entropy_color(app.entropy);

    let pct = (app.entropy / 128.0 * 100.0).clamp(0.0, 100.0) as u16;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(
            " Strength ",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Compact: label on first line, thin gauge on second
    let label_line = Line::from(vec![
        Span::styled(
            format!("  {label}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  —  {:.1} bits", app.entropy),
            Style::default().fg(DIM),
        ),
    ]);

    let gauge_height = if inner.height > 2 { 1 } else { 0 };

    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(gauge_height),
        ])
        .split(inner);

    let label_para = Paragraph::new(label_line);
    f.render_widget(label_para, content_chunks[0]);

    if gauge_height > 0 {
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(color).bg(Color::Rgb(30, 30, 46)))
            .percent(pct)
            .label("");
        f.render_widget(gauge, content_chunks[1]);
    }
}

fn draw_hints(f: &mut Frame, area: Rect) {
    let hints = Line::from(vec![
        Span::styled(" Enter", Style::default().fg(GREEN)),
        Span::styled(" Generate  ", Style::default().fg(DIM)),
        Span::styled(" y", Style::default().fg(GREEN)),
        Span::styled(" Copy  ", Style::default().fg(DIM)),
        Span::styled(" Tab", Style::default().fg(GREEN)),
        Span::styled(" Mode  ", Style::default().fg(DIM)),
        Span::styled(" ↑↓", Style::default().fg(GREEN)),
        Span::styled(" Navigate  ", Style::default().fg(DIM)),
        Span::styled(" ←→", Style::default().fg(GREEN)),
        Span::styled(" Adjust  ", Style::default().fg(DIM)),
        Span::styled(" q", Style::default().fg(GREEN)),
        Span::styled(" Quit", Style::default().fg(DIM)),
    ]);

    let para = Paragraph::new(hints);
    f.render_widget(
        para,
        Rect {
            x: area.x + 1,
            y: area.bottom().saturating_sub(1),
            width: area.width.saturating_sub(2),
            height: 1,
        },
    );
}

fn draw_random_options(app: &App) -> Vec<Line<'static>> {
    let focused = |i: usize| matches!(app.focus, Focus::RandomOpt(j) if j == i);
    let check = |val: bool| -> &'static str {
        if val { "[X]" } else { "[ ]" }
    };

    vec![
        make_option(focused(0), "Length", &format!("{}", app.random_cfg.length)),
        make_option(focused(1), "Uppercase", check(app.random_cfg.uppercase)),
        make_option(focused(2), "Lowercase", check(app.random_cfg.lowercase)),
        make_option(focused(3), "Numbers", check(app.random_cfg.numbers)),
        make_option(focused(4), "Symbols", check(app.random_cfg.symbols)),
        make_option(focused(5), "No ambiguous", check(app.random_cfg.exclude_ambiguous)),
    ]
}

fn draw_memorable_options(app: &App) -> Vec<Line<'static>> {
    let focused = |i: usize| matches!(app.focus, Focus::MemorableOpt(j) if j == i);
    let check = |val: bool| -> &'static str {
        if val { "[X]" } else { "[ ]" }
    };

    let sep_display = if app.editing_separator {
        format!(
            "{}█",
            app.separator_custom.as_deref().unwrap_or("")
        )
    } else if let Some(ref s) = app.separator_custom {
        format!("\"{s}\"")
    } else {
        let presets = separator_presets();
        if presets[app.separator_idx].is_empty() {
            "(none)".into()
        } else {
            format!("\"{}\"", presets[app.separator_idx])
        }
    };

    vec![
        make_option(focused(0), "Words", &format!("{}", app.memorable_cfg.word_count)),
        make_option(focused(1), "Separator", &sep_display),
        make_option(focused(2), "Capitalize", check(app.memorable_cfg.capitalize)),
        make_option(focused(3), "Add Numbers", check(app.memorable_cfg.add_numbers)),
        make_option(focused(4), "Truncate", check(app.memorable_cfg.truncate)),
    ]
}

fn make_option<'a>(focused: bool, label: &str, value: &str) -> Line<'a> {
    let indicator = if focused { "> " } else { "  " };
    Line::from(vec![
        Span::styled(
            indicator,
            Style::default().fg(if focused { GREEN } else { DIM }),
        ),
        Span::styled(
            format!("{label}: "),
            Style::default().fg(TEXT),
        ),
        Span::styled(
            value.to_string(),
            Style::default().fg(if focused { GREEN } else { DIM }).add_modifier(
                if focused { Modifier::BOLD } else { Modifier::empty() },
            ),
        ),
    ])
}

// Catppuccin Mocha palette
const BORDER: Color = Color::Rgb(88, 91, 112);
const TEXT: Color = Color::Rgb(205, 214, 244);
const DIM: Color = Color::Rgb(166, 173, 200);
const GREEN: Color = Color::Rgb(166, 227, 161);
const ORANGE: Color = Color::Rgb(250, 179, 135);
const BLUE: Color = Color::Rgb(137, 180, 250);

// Smooth entropy color: red(0) → orange(30) → yellow(50) → green(80+)
fn entropy_color(entropy: f64) -> Color {
    let (r1, g1, b1, r2, g2, b2, t) = if entropy < 30.0 {
        // Red → Orange
        let t = (entropy / 30.0).clamp(0.0, 1.0);
        (243, 139, 168, 250, 179, 135, t)
    } else if entropy < 50.0 {
        // Orange → Yellow
        let t = ((entropy - 30.0) / 20.0).clamp(0.0, 1.0);
        (250, 179, 135, 249, 226, 175, t)
    } else if entropy < 80.0 {
        // Yellow → Green
        let t = ((entropy - 50.0) / 30.0).clamp(0.0, 1.0);
        (249, 226, 175, 166, 227, 161, t)
    } else {
        return GREEN;
    };
    Color::Rgb(
        (r1 as f64 + (r2 as f64 - r1 as f64) * t) as u8,
        (g1 as f64 + (g2 as f64 - g1 as f64) * t) as u8,
        (b1 as f64 + (b2 as f64 - b1 as f64) * t) as u8,
    )
}
