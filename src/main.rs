use std::cmp::{max, min};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use chrono::Local;
use clap::{Arg, Command as ClapCommand};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Attribute, Print, SetAttribute},
    terminal::{
        self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
        SetTitle,
    },

};
use anyhow::Result;
use serde::Deserialize;

/// Default ASCII art (fallback when user files don't exist)
const DEFAULT_ASCII_ART: &str = include_str!("../default_ascii.txt");

/// TOML Config structures
#[derive(Debug, Deserialize)]
struct TomlConfig {
    durations: Durations,
    paths: Paths,
    ascii_art: AsciiArtConfig,
}

#[derive(Debug, Deserialize)]
struct Durations {
    work_min: u64,
    short_min: u64,
    long_min: u64,
    long_every: u64,
}

#[derive(Debug, Deserialize)]
struct Paths {
    log_dir: String,
    ascii_dir: String,
    log_filename: String,
}

#[derive(Debug, Deserialize)]
struct AsciiArtConfig {
    enabled: bool,
    idle_file: String,
    work_file: String,
    short_file: String,
    long_file: String,
}

/// Load config from config.toml or use hardcoded defaults
fn load_toml_config() -> TomlConfig {
    let config_path = PathBuf::from("config.toml");
    
    if config_path.exists() {
        match fs::read_to_string(&config_path) {
            Ok(contents) => {
                match toml::from_str::<TomlConfig>(&contents) {
                    Ok(config) => return config,
                    Err(e) => {
                        eprintln!("Warning: Failed to parse config.toml: {}. Using defaults.", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to read config.toml: {}. Using defaults.", e);
            }
        }
    }
    
    // Default config if file doesn't exist or failed to parse
    TomlConfig {
        durations: Durations {
            work_min: 25,
            short_min: 5,
            long_min: 15,
            long_every: 4,
        },
        paths: Paths {
            log_dir: ".ptimer/logs".to_string(),
            ascii_dir: ".ptimer/ascii_art".to_string(),
            log_filename: "pomodoro.csv".to_string(),
        },
        ascii_art: AsciiArtConfig {
            enabled: true,
            idle_file: "idle.txt".to_string(),
            work_file: "work.txt".to_string(),
            short_file: "short.txt".to_string(),
            long_file: "long.txt".to_string(),
        },
    }
}

fn get_home_dir() -> PathBuf {
    dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Work,
    Short,
    Long,
}

impl Phase {
    fn label(self) -> &'static str {
        match self {
            Phase::Idle => "Press [s] to start WORK",
            Phase::Work => "WORK",
            Phase::Short => "PAUSE",
            Phase::Long => "LONG PAUSE",
        }
    }
    
    fn ascii_filename(self, config: &AsciiArtConfig) -> &str {
        match self {
            Phase::Idle => &config.idle_file,
            Phase::Work => &config.work_file,
            Phase::Short => &config.short_file,
            Phase::Long => &config.long_file,
        }
    }
}

#[derive(Debug)]
struct Config {
    work_sec: u64,
    short_sec: u64,
    long_sec: u64,
    long_every: u64,
    ascii_dir: PathBuf,
    ascii_config: AsciiArtConfig,
    log_path: PathBuf,
}

#[derive(Debug)]
struct State {
    phase: Phase,
    remaining: f64,            // seconds, fractional
    started_at: Option<Instant>,
    paused: bool,
    sessions_completed: u64,   // # completed WORK sessions
    ascii_art: Option<String>,
    last_size: (u16, u16),
}

/// Read ASCII art for a specific phase
fn read_ascii_for_phase(phase: Phase, ascii_dir: &PathBuf, ascii_config: &AsciiArtConfig) -> Option<String> {
    if !ascii_config.enabled {
        return None;
    }
    
    let filename = phase.ascii_filename(ascii_config);
    let path = ascii_dir.join(filename);
    
    match fs::read_to_string(&path) {
        Ok(s) => Some(s.trim_end_matches('\n').to_string()),
        Err(_) => {
            // Use default ASCII art as fallback
            Some(DEFAULT_ASCII_ART.trim_end_matches('\n').to_string())
        }
    }
}

/// Update state's ASCII art when phase changes
fn update_phase(state: &mut State, new_phase: Phase, cfg: &Config) {
    state.phase = new_phase;
    state.ascii_art = read_ascii_for_phase(new_phase, &cfg.ascii_dir, &cfg.ascii_config);
}

/// Best-effort: ring bell and try to raise terminal.
fn alert_bring_to_front() {
    // Terminal bell
    print!("\x07");
    let _ = io::stdout().flush();

    // Set a unique title (helps some WMs find the window)
    let unique = format!("Pomodoro Alert {}", Local::now().timestamp());
    let _ = execute!(io::stdout(), SetTitle(unique.clone()));

    // Platform-specific nudges
    #[cfg(target_os = "macos")]
    {
        // Try to activate Apple Terminal and iTerm
        let _ = Command::new("osascript")
            .arg("-e")
            .arg(r#"tell application "Terminal" to activate"#)
            .status();
        let _ = Command::new("osascript")
            .arg("-e")
            .arg(r#"tell application "iTerm" to activate"#)
            .status();
    }

    #[cfg(target_os = "linux")]
    {
        // If available, try wmctrl & notify-send
        if which::which("wmctrl").is_ok() {
            let _ = Command::new("wmctrl").arg("-a").arg(unique).status();
        }
        if which::which("notify-send").is_ok() {
            let _ = Command::new("notify-send")
                .args(["-u", "critical", "Pomodoro", "Time's up!"])
                .status();
        }
    }

    // Windows: bell is often enough; bringing windows to foreground programmatically is restricted.
}

/// Write a CSV log row.
fn log_phase(log_path: &PathBuf, phase: Phase, configured_secs: u64, started_at: Option<Instant>) {
    if started_at.is_none() {
        return;
    }
    let elapsed = started_at.unwrap().elapsed().as_secs();
    let duration = if configured_secs == 0 { elapsed } else { min(elapsed, configured_secs) };

    let ts = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let phase_str = match phase {
        Phase::Idle => "IDLE",
        Phase::Work => "WORK",
        Phase::Short => "PAUSE",
        Phase::Long => "LONG PAUSE",
    };

    // Ensure file exists with header
    if !log_path.exists() {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(f, "timestamp,phase,duration_sec,notes");
        }
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = writeln!(f, "{},{},{},completed", ts, phase_str, duration);
    }
}

fn human_mmss(total_seconds: f64) -> String {
    let total = max(0, total_seconds.floor() as i64) as u64;
    let m = total / 60;
    let s = total % 60;
    format!("{:02}:{:02}", m, s)
}

fn parse_cli() -> Config {
    // Load TOML config first
    let toml_cfg = load_toml_config();
    
    let m = ClapCommand::new("ptimer")
        .about("Terminal Pomodoro (reactive, logs sessions, optional ASCII art)")
        .arg(
            Arg::new("work")
                .long("work")
                .short('w')
                .num_args(1)
                .value_name("MIN")
                .help(&format!("Work duration in minutes (default {})", toml_cfg.durations.work_min)),
        )
        .arg(
            Arg::new("short")
                .long("short")
                .short('s')
                .num_args(1)
                .value_name("MIN")
                .help(&format!("Short break in minutes (default {})", toml_cfg.durations.short_min)),
        )
        .arg(
            Arg::new("long")
                .long("long")
                .short('l')
                .num_args(1)
                .value_name("MIN")
                .help(&format!("Long break in minutes (default {})", toml_cfg.durations.long_min)),
        )
        .arg(
            Arg::new("long_every")
                .long("long-every")
                .short('n')
                .num_args(1)
                .value_name("N")
                .help(&format!("Use a long break every N work sessions (default {})", toml_cfg.durations.long_every)),
        )
        .arg(
            Arg::new("log")
                .long("log")
                .num_args(1)
                .value_name("PATH")
                .help("Override log directory path"),
        )
        .arg(
            Arg::new("ascii")
                .long("ascii")
                .num_args(1)
                .value_name("PATH")
                .help("Override ASCII art directory path"),
        )
        .get_matches();

    let work_min = m
        .get_one::<String>("work")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(toml_cfg.durations.work_min);

    let short_min = m
        .get_one::<String>("short")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(toml_cfg.durations.short_min);

    let long_min = m
        .get_one::<String>("long")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(toml_cfg.durations.long_min);

    let long_every = m
        .get_one::<String>("long_every")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(toml_cfg.durations.long_every);

    // Set up paths (in home directory by default)
    let home = get_home_dir();
    
    let log_dir = m
        .get_one::<String>("log")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(&toml_cfg.paths.log_dir));
    
    let ascii_dir = m
        .get_one::<String>("ascii")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(&toml_cfg.paths.ascii_dir));

    // Create directories if they don't exist
    if let Err(e) = fs::create_dir_all(&log_dir) {
        eprintln!("Warning: Failed to create log directory: {}", e);
    }
    
    if let Err(e) = fs::create_dir_all(&ascii_dir) {
        eprintln!("Warning: Failed to create ASCII art directory: {}", e);
    }

    let log_path = log_dir.join(&toml_cfg.paths.log_filename);

    Config {
        work_sec: work_min * 60,
        short_sec: short_min * 60,
        long_sec: long_min * 60,
        long_every,
        ascii_dir,
        ascii_config: toml_cfg.ascii_art,
        log_path,
    }
}

/// Simple, resilient draw routine using crossterm.
fn draw(state: &State, cfg: &Config) -> Result<()> {
    let mut out = io::stdout();
    let (cols, rows) = state.last_size;

    // Clear
    queue!(out, cursor::Hide, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

    // Constrain to max box size
    const MAX_BOX_WIDTH: u16 = 150;
    const MAX_BOX_HEIGHT: u16 = 100;
    
    let _box_width = min(MAX_BOX_WIDTH, cols);
    let box_height = min(MAX_BOX_HEIGHT, rows);
    
    // Position box on the left side of terminal
    let box_offset_x = 0u16;
    let box_offset_y = 0u16;
    
    let pad = 2u16;
    
    // Prepare text content
    let label = format!("[ {} ]", state.phase.label());
    let timer = if state.phase == Phase::Idle {
        "--:--".to_string()
    } else {
        human_mmss(state.remaining)
    };

    // Measure ASCII art
    let art_lines: Vec<&str> = state
        .ascii_art
        .as_deref()
        .map(|s| s.lines().collect())
        .unwrap_or_else(|| Vec::new());
    // Use char count, not byte length, for proper Unicode handling
    let art_width = art_lines.iter().map(|l| l.chars().count() as u16).max().unwrap_or(0);
    let art_height = art_lines.len() as u16;

    // Progress bar width
    let bar_w = 60usize;
    let bar_total_width = (bar_w + 2) as u16; // +2 for the brackets
    
    // Position elements
    let bar_x = box_offset_x + pad;
    
    // Title (centered relative to progress bar)
    let title = "PTIMER";
    let title_x = bar_x + ((bar_total_width as i32 - title.len() as i32) / 2).max(0) as u16;
    queue!(
        out,
        cursor::MoveTo(title_x, box_offset_y + pad),
        SetAttribute(Attribute::Bold),
        Print(title),
        SetAttribute(Attribute::Reset)
    )?;
    
    // Position timer/action on the left
    let start_x = bar_x;
    let content_y = box_offset_y + pad + 3;
    
    // Vertically center the text within the ASCII art height
    let text_block_height = 3u16; // label + 1 space + timer
    let text_offset_y = if art_height > text_block_height {
        (art_height - text_block_height) / 2
    } else {
        0
    };
    
    // Draw phase label (left-aligned)
    queue!(
        out,
        cursor::MoveTo(start_x, content_y + text_offset_y),
        SetAttribute(Attribute::Reverse),
        Print(&label),
        SetAttribute(Attribute::Reset)
    )?;
    
    // Draw timer (centered relative to the label above it)
    let timer_offset = (label.len() as i32 - timer.len() as i32) / 2;
    let timer_x = if timer_offset > 0 {
        start_x + timer_offset as u16
    } else {
        start_x
    };
    queue!(
        out,
        cursor::MoveTo(timer_x, content_y + text_offset_y + 2),
        SetAttribute(Attribute::Bold),
        Print(&timer),
        SetAttribute(Attribute::Reset)
    )?;

    // Draw ASCII art (if available) aligned to the right of the box
    if art_width > 0 {
        // Right-align ASCII art within the progress bar width
        let art_right_edge = bar_x + bar_total_width;
        let art_x = art_right_edge.saturating_sub(art_width);
        
        for (i, line) in art_lines.iter().enumerate() {
            let y = content_y + i as u16;
            if y >= box_offset_y + box_height.saturating_sub(pad) {
                break;
            }
            queue!(out, cursor::MoveTo(art_x, y), Print(line))?;
        }
    }

    // Progress bar (underneath the combined content, left-aligned within box)
    let bar_y = content_y + max(text_block_height + text_offset_y, art_height) + 2;
    
    let total = match state.phase {
        Phase::Work => cfg.work_sec,
        Phase::Short => cfg.short_sec,
        Phase::Long => cfg.long_sec,
        Phase::Idle => 1,
    } as f64;

    let pct = if total <= 0.0 {
        0.0
    } else {
        ((total - state.remaining) / total).clamp(0.0, 1.0)
    };
    let filled = (pct * bar_w as f64).round() as usize;
    let bar = format!(
        "[{}{}]",
        "#".repeat(filled),
        "-".repeat(bar_w.saturating_sub(filled))
    );
    queue!(out, cursor::MoveTo(bar_x, bar_y), Print(&bar))?;

    // Footer info (below progress bar, left-aligned within box)
    let footer_y = bar_y + 2;
    let info = vec![
        format!("sessions: {}  (long every {})", state.sessions_completed, cfg.long_every),
        "keys: [s]tart work  [p]ause/resume  [q]uit".to_string(),
        format!("log: {}", cfg.log_path.display()),
        format!("ascii: {}", cfg.ascii_dir.display()),
    ];
    for (i, line) in info.iter().enumerate() {
        queue!(out, cursor::MoveTo(box_offset_x + pad, footer_y + i as u16), Print(&line))?;
    }

    out.flush()?;
    Ok(())
}

fn main() -> Result<()> {
    // Parse CLI & build config
    let cfg = parse_cli();

    // Pre-create log file with header if missing
    if !cfg.log_path.exists() {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&cfg.log_path) {
            let _ = writeln!(f, "timestamp,phase,duration_sec,notes");
        }
    }

    // Initial state
    let (cols, rows) = terminal::size()?;
    let mut state = State {
        phase: Phase::Idle,
        remaining: 0.0,
        started_at: None,
        paused: false,
        sessions_completed: 0,
        ascii_art: read_ascii_for_phase(Phase::Idle, &cfg.ascii_dir, &cfg.ascii_config),
        last_size: (cols, rows),
    };

    // Terminal setup
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide, SetTitle("ptimer"))?;

    terminal::enable_raw_mode()?;

    // Draw once
    let _ = draw(&state, &cfg);

    // Main loop
    let tick = Duration::from_millis(250);
    let mut last_drawn_second: i64 = -1;
    'outer: loop {
        // Handle input or resize
        if event::poll(tick)? {
            match event::read()? {
                Event::Key(KeyEvent { code, modifiers, .. }) => {
                    match code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            // Log current phase if it was running
                            match state.phase {
                                Phase::Work => log_phase(&cfg.log_path, Phase::Work, cfg.work_sec, state.started_at),
                                Phase::Short => log_phase(&cfg.log_path, Phase::Short, cfg.short_sec, state.started_at),
                                Phase::Long => log_phase(&cfg.log_path, Phase::Long, cfg.long_sec, state.started_at),
                                Phase::Idle => {}
                            }
                            break 'outer;
                        }
                        KeyCode::Char('s') => {
                            if matches!(state.phase, Phase::Idle | Phase::Short | Phase::Long) {
                                update_phase(&mut state, Phase::Work, &cfg);
                                state.remaining = cfg.work_sec as f64;
                                state.started_at = Some(Instant::now());
                                state.paused = false;
                                let _ = draw(&state, &cfg);
                            }
                        }
                        KeyCode::Char('p') => {
                            if state.phase != Phase::Idle {
                                state.paused = !state.paused;
                                let _ = draw(&state, &cfg);
                            }
                        }
                        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                            // Graceful exit on Ctrl+C
                            break 'outer;
                        }
                        _ => {}
                    }
                }
                Event::Resize(c, r) => {
                    state.last_size = (c, r);
                    let _ = draw(&state, &cfg);
                }
                _ => {}
            }
        }

        // Tick countdown
        if !state.paused {
            match state.phase {
                Phase::Work | Phase::Short | Phase::Long => {
                    state.remaining -= tick.as_secs_f64();
                    if state.remaining <= 0.0 {
                        // Phase complete: log + alert + transition
                        let finished_phase = state.phase;
                        let cfg_secs = match finished_phase {
                            Phase::Work => cfg.work_sec,
                            Phase::Short => cfg.short_sec,
                            Phase::Long => cfg.long_sec,
                            Phase::Idle => 0,
                        };
                        log_phase(&cfg.log_path, finished_phase, cfg_secs, state.started_at);
                        alert_bring_to_front();

                        if finished_phase == Phase::Work {
                            state.sessions_completed += 1;
                            let long_break = state.sessions_completed % cfg.long_every == 0;
                            let next_phase = if long_break { Phase::Long } else { Phase::Short };
                            update_phase(&mut state, next_phase, &cfg);
                            state.remaining = if long_break { cfg.long_sec as f64 } else { cfg.short_sec as f64 };
                            state.started_at = Some(Instant::now());
                            state.paused = false;
                        } else {
                            // After any break, go idle and wait for manual 's'
                            update_phase(&mut state, Phase::Idle, &cfg);
                            state.remaining = 0.0;
                            state.started_at = None;
                            state.paused = false;
                        }
                        last_drawn_second = -1; // Force redraw on phase change
                        let _ = draw(&state, &cfg);
                    } else {
                        // Only redraw when the displayed second changes
                        let current_second = state.remaining.floor() as i64;
                        if current_second != last_drawn_second {
                            last_drawn_second = current_second;
                            let _ = draw(&state, &cfg);
                        }
                    }
                }
                Phase::Idle => {} // nothing
            }
        }
    }

    // Cleanup terminal
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, cursor::Show)?;
    Ok(())
}
