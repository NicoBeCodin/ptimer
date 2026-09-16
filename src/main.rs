use std::cmp::{max, min};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::Local;
use clap::{Arg, Command as ClapCommand};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use serde::{Deserialize, Serialize};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

mod animation;

const DEFAULT_ASCII_ART: &str = include_str!("../default_ascii.txt");

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

fn default_toml_config() -> TomlConfig {
    TomlConfig {
        durations: Durations {
            work_min: 25,
            short_min: 5,
            long_min: 15,
            long_every: 4,
        },
        paths: Paths {
            log_dir: ".ptimer/logs".into(),
            ascii_dir: ".ptimer/ascii_art".into(),
            log_filename: "pomodoro.csv".into(),
        },
        ascii_art: AsciiArtConfig {
            enabled: true,
            idle_file: "idle.txt".into(),
            work_file: "work.txt".into(),
            short_file: "short.txt".into(),
            long_file: "long.txt".into(),
        },
    }
}

fn load_toml_config() -> TomlConfig {
    fs::read_to_string("config.toml")
        .ok()
        .and_then(|contents| toml::from_str(&contents).ok())
        .unwrap_or_else(default_toml_config)
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
            Self::Idle => "READY",
            Self::Work => "WORK",
            Self::Short => "SHORT BREAK",
            Self::Long => "LONG BREAK",
        }
    }

    fn log_label(self) -> &'static str {
        match self {
            Self::Idle => "IDLE",
            Self::Work => "WORK",
            Self::Short => "PAUSE",
            Self::Long => "LONG PAUSE",
        }
    }

    fn ascii_filename(self, config: &AsciiArtConfig) -> &str {
        match self {
            Self::Idle => &config.idle_file,
            Self::Work => &config.work_file,
            Self::Short => &config.short_file,
            Self::Long => &config.long_file,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ArtStyle {
    Custom,
    OriginalCat,
    Tomato,
    Coffee,
    Cat,
    Focus,
    Sky,
    Plant,
    Hourglass,
    Fireplace,
    Aquarium,
    Rocket,
    GrandCat,
    GrandCastle,
    GrandCosmos,
    Off,
}

impl ArtStyle {
    const ALL: [Self; 16] = [
        Self::Custom,
        Self::OriginalCat,
        Self::Tomato,
        Self::Coffee,
        Self::Cat,
        Self::Focus,
        Self::Sky,
        Self::Plant,
        Self::Hourglass,
        Self::Fireplace,
        Self::Aquarium,
        Self::Rocket,
        Self::GrandCat,
        Self::GrandCastle,
        Self::GrandCosmos,
        Self::Off,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::OriginalCat => "original cat",
            Self::Tomato => "tomato",
            Self::Coffee => "coffee",
            Self::Cat => "small cat",
            Self::Focus => "focus orb",
            Self::Sky => "drifting sky",
            Self::Plant => "growing plant",
            Self::Hourglass => "hourglass",
            Self::Fireplace => "fireplace",
            Self::Aquarium => "aquarium",
            Self::Rocket => "rocket",
            Self::GrandCat => "GRAND cat",
            Self::GrandCastle => "GRAND castle",
            Self::GrandCosmos => "GRAND cosmos",
            Self::Off => "off",
        }
    }
    fn cycle(self, delta: i32) -> Self {
        cycle_value(&Self::ALL, self, delta)
    }

    fn frame_interval(self) -> Option<Duration> {
        match self {
            Self::Custom | Self::OriginalCat | Self::Off => None,
            Self::Tomato => Some(Duration::from_millis(420)),
            Self::Coffee => Some(Duration::from_millis(240)),
            Self::Cat => Some(Duration::from_millis(360)),
            Self::Focus => Some(Duration::from_millis(100)),
            Self::Sky => Some(Duration::from_millis(180)),
            Self::Plant => Some(Duration::from_millis(650)),
            Self::Hourglass => Some(Duration::from_millis(500)),
            Self::Fireplace => Some(Duration::from_millis(180)),
            Self::Aquarium => Some(Duration::from_millis(160)),
            Self::Rocket => Some(Duration::from_millis(140)),
            Self::GrandCat => Some(Duration::from_millis(300)),
            Self::GrandCastle => Some(Duration::from_millis(220)),
            Self::GrandCosmos => Some(Duration::from_millis(140)),
        }
    }

    fn is_grand(self) -> bool {
        matches!(self, Self::GrandCat | Self::GrandCastle | Self::GrandCosmos)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ClockStyle {
    Digital,
    Classic,
    Minimal,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum NotificationStyle {
    Off,
    Normal,
    Urgent,
}

impl NotificationStyle {
    const ALL: [Self; 3] = [Self::Off, Self::Normal, Self::Urgent];
    fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Normal => "normal",
            Self::Urgent => "urgent",
        }
    }
    fn cycle(self, delta: i32) -> Self {
        cycle_value(&Self::ALL, self, delta)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SoundStyle {
    Off,
    Bell,
    Chime,
}

impl SoundStyle {
    const ALL: [Self; 3] = [Self::Off, Self::Bell, Self::Chime];
    fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Bell => "terminal bell",
            Self::Chime => "system chime",
        }
    }
    fn cycle(self, delta: i32) -> Self {
        cycle_value(&Self::ALL, self, delta)
    }
}

impl ClockStyle {
    const ALL: [Self; 3] = [Self::Digital, Self::Classic, Self::Minimal];
    fn label(self) -> &'static str {
        match self {
            Self::Digital => "digital",
            Self::Classic => "classic",
            Self::Minimal => "minimal",
        }
    }
    fn cycle(self, delta: i32) -> Self {
        cycle_value(&Self::ALL, self, delta)
    }
}

fn cycle_value<T: Copy + PartialEq>(values: &[T], current: T, delta: i32) -> T {
    let index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0) as i32;
    values[(index + delta).rem_euclid(values.len() as i32) as usize]
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    work_min: u64,
    short_min: u64,
    long_min: u64,
    long_every: u64,
    art_style: ArtStyle,
    animations_enabled: bool,
    clock_style: ClockStyle,
    notifications: NotificationStyle,
    sound: SoundStyle,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            work_min: 25,
            short_min: 5,
            long_min: 15,
            long_every: 4,
            art_style: ArtStyle::Custom,
            animations_enabled: true,
            clock_style: ClockStyle::Digital,
            notifications: NotificationStyle::Urgent,
            sound: SoundStyle::Bell,
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
    preferences_path: PathBuf,
    art_style: ArtStyle,
    animations_enabled: bool,
    clock_style: ClockStyle,
    notifications: NotificationStyle,
    sound: SoundStyle,
}

impl Config {
    fn preferences(&self) -> Preferences {
        Preferences {
            work_min: self.work_sec / 60,
            short_min: self.short_sec / 60,
            long_min: self.long_sec / 60,
            long_every: self.long_every,
            art_style: self.art_style,
            animations_enabled: self.animations_enabled,
            clock_style: self.clock_style,
            notifications: self.notifications,
            sound: self.sound,
        }
    }

    fn save(&self) -> io::Result<()> {
        if let Some(parent) = self.preferences_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(&self.preferences()).map_err(io::Error::other)?;
        let temporary = self.preferences_path.with_extension("toml.tmp");
        fs::write(&temporary, text)?;
        fs::rename(temporary, &self.preferences_path)
    }
}

#[derive(Debug)]
struct State {
    phase: Phase,
    remaining: f64,
    started_at: Option<Instant>,
    paused: bool,
    sessions_completed: u64,
    ascii_art: Option<String>,
    last_size: (u16, u16),
    settings_open: bool,
    selected_setting: usize,
    save_message: Option<String>,
    animation_frame: usize,
}

fn read_ascii_for_phase(phase: Phase, cfg: &Config, frame: usize) -> Option<String> {
    let art = match cfg.art_style {
        ArtStyle::Off => return None,
        ArtStyle::OriginalCat => DEFAULT_ASCII_ART.to_string(),
        ArtStyle::Tomato => animation::tomato(frame),
        ArtStyle::Coffee => animation::coffee(frame),
        ArtStyle::Cat => animation::cat(frame),
        ArtStyle::Focus => animation::focus(frame),
        ArtStyle::Sky => animation::sky(frame),
        ArtStyle::Plant => animation::plant(frame),
        ArtStyle::Hourglass => animation::hourglass(frame),
        ArtStyle::Fireplace => animation::fireplace(frame),
        ArtStyle::Aquarium => animation::aquarium(frame),
        ArtStyle::Rocket => animation::rocket(frame),
        ArtStyle::GrandCat => animation::grand_cat(frame),
        ArtStyle::GrandCastle => animation::grand_castle(frame),
        ArtStyle::GrandCosmos => animation::grand_cosmos(frame),
        ArtStyle::Custom if !cfg.ascii_config.enabled => return None,
        ArtStyle::Custom => {
            fs::read_to_string(cfg.ascii_dir.join(phase.ascii_filename(&cfg.ascii_config)))
                .unwrap_or_else(|_| DEFAULT_ASCII_ART.to_string())
        }
    };
    Some(art.trim_end_matches('\n').to_string())
}

fn update_phase(state: &mut State, new_phase: Phase, cfg: &Config) {
    state.phase = new_phase;
    state.animation_frame = 0;
    state.ascii_art = read_ascii_for_phase(new_phase, cfg, 0);
}

fn play_sound(style: SoundStyle) {
    match style {
        SoundStyle::Off => {}
        SoundStyle::Bell => {
            print!("\x07");
            let _ = io::stdout().flush();
        }
        SoundStyle::Chime => {
            let played = which::which("canberra-gtk-play").is_ok()
                && std::process::Command::new("canberra-gtk-play")
                    .args(["--id", "complete", "--description", "PTimer complete"])
                    .spawn()
                    .is_ok();
            if !played {
                print!("\x07");
                let _ = io::stdout().flush();
            }
        }
    }
}

fn send_desktop_notification(style: NotificationStyle, message: &str) {
    if style == NotificationStyle::Off || which::which("notify-send").is_err() {
        return;
    }
    let urgency = if style == NotificationStyle::Urgent {
        "critical"
    } else {
        "normal"
    };
    let _ = std::process::Command::new("notify-send")
        .args(["-u", urgency, "PTimer", message])
        .spawn();
}

fn alert_user(phase: Phase, cfg: &Config) {
    play_sound(cfg.sound);
    let msg = format!(
        "\n\n\n  =============== {} ENDED ===============  \n\n\n",
        phase.log_label()
    );
    send_desktop_notification(cfg.notifications, &msg);
}

fn test_alert(cfg: &Config) {
    play_sound(cfg.sound);
    send_desktop_notification(cfg.notifications, "Notifications are ready.");
}

fn log_phase(log_path: &Path, phase: Phase, configured_secs: u64, started_at: Option<Instant>) {
    let Some(started_at) = started_at else { return };
    let elapsed = started_at.elapsed().as_secs();
    let duration = if configured_secs == 0 {
        elapsed
    } else {
        min(elapsed, configured_secs)
    };
    let ts = Local::now().format("%Y-%m-%dT%H:%M:%S");
    if !log_path.exists()
        && let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path)
    {
        let _ = writeln!(file, "timestamp,phase,duration_sec,notes");
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = writeln!(file, "{},{},{},completed", ts, phase.log_label(), duration);
    }
}

fn human_mmss(total_seconds: f64) -> String {
    let total = max(0, total_seconds.ceil() as i64) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

fn load_preferences(path: &Path, defaults: Preferences) -> Preferences {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or(defaults)
}

fn parse_cli() -> Config {
    let toml_cfg = load_toml_config();
    let preferences_path = dirs_next::config_dir()
        .unwrap_or_else(|| get_home_dir().join(".config"))
        .join("ptimer/settings.toml");
    let defaults = Preferences {
        work_min: toml_cfg.durations.work_min,
        short_min: toml_cfg.durations.short_min,
        long_min: toml_cfg.durations.long_min,
        long_every: toml_cfg.durations.long_every,
        ..Preferences::default()
    };
    let preferences = load_preferences(&preferences_path, defaults);
    let matches = ClapCommand::new("ptimer")
        .about("A responsive terminal Pomodoro timer")
        .arg(
            Arg::new("work")
                .long("work")
                .short('w')
                .num_args(1)
                .value_name("MIN"),
        )
        .arg(
            Arg::new("short")
                .long("short")
                .short('s')
                .num_args(1)
                .value_name("MIN"),
        )
        .arg(
            Arg::new("long")
                .long("long")
                .short('l')
                .num_args(1)
                .value_name("MIN"),
        )
        .arg(
            Arg::new("long_every")
                .long("long-every")
                .short('n')
                .num_args(1)
                .value_name("N"),
        )
        .arg(Arg::new("log").long("log").num_args(1).value_name("PATH"))
        .arg(
            Arg::new("ascii")
                .long("ascii")
                .num_args(1)
                .value_name("PATH"),
        )
        .get_matches();
    let number = |name: &str, fallback| {
        matches
            .get_one::<String>(name)
            .and_then(|s| s.parse().ok())
            .unwrap_or(fallback)
    };
    let home = get_home_dir();
    let log_dir = matches
        .get_one::<String>("log")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(&toml_cfg.paths.log_dir));
    let ascii_dir = matches
        .get_one::<String>("ascii")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(&toml_cfg.paths.ascii_dir));
    let _ = fs::create_dir_all(&log_dir);
    let _ = fs::create_dir_all(&ascii_dir);
    Config {
        work_sec: number("work", preferences.work_min).max(1) * 60,
        short_sec: number("short", preferences.short_min).max(1) * 60,
        long_sec: number("long", preferences.long_min).max(1) * 60,
        long_every: number("long_every", preferences.long_every).max(1),
        ascii_dir,
        ascii_config: toml_cfg.ascii_art,
        log_path: log_dir.join(toml_cfg.paths.log_filename),
        preferences_path,
        art_style: preferences.art_style,
        animations_enabled: preferences.animations_enabled,
        clock_style: preferences.clock_style,
        notifications: preferences.notifications,
        sound: preferences.sound,
    }
}

const DIGITS: [[&str; 5]; 11] = [
    [" __ ", "|  |", "|  |", "|  |", "|__|"],
    ["    ", "   |", "   |", "   |", "   |"],
    [" __ ", "   |", " __|", "|   ", "|__ "],
    [" __ ", "   |", " __|", "   |", " __|"],
    ["    ", "|  |", "|__|", "   |", "   |"],
    [" __ ", "|   ", "|__ ", "   |", " __|"],
    [" __ ", "|   ", "|__ ", "|  |", "|__|"],
    [" __ ", "   |", "   |", "   |", "   |"],
    [" __ ", "|  |", "|__|", "|  |", "|__|"],
    [" __ ", "|  |", "|__|", "   |", " __|"],
    [" ", "o", " ", "o", " "],
];

fn clock_lines(state: &State, cfg: &Config, available_width: u16) -> Vec<String> {
    let plain = if state.phase == Phase::Idle {
        "--:--".to_string()
    } else {
        human_mmss(state.remaining)
    };
    match cfg.clock_style {
        ClockStyle::Classic => vec![plain],
        ClockStyle::Minimal => {
            if state.phase == Phase::Idle {
                vec!["ready".into()]
            } else {
                let total = max(0, state.remaining.ceil() as i64) as u64;
                vec![format!("{}m {:02}s", total / 60, total % 60)]
            }
        }
        ClockStyle::Digital if state.phase != Phase::Idle && available_width >= 25 => {
            let indexes: Vec<usize> = plain
                .chars()
                .map(|c| {
                    if c == ':' {
                        10
                    } else {
                        c.to_digit(10).unwrap_or(0) as usize
                    }
                })
                .collect();
            (0..5)
                .map(|row| {
                    indexes
                        .iter()
                        .map(|index| DIGITS[*index][row])
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim_end()
                        .to_string()
                })
                .collect()
        }
        ClockStyle::Digital => vec![plain],
    }
}

fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

fn truncate_to_width(text: &str, width: usize) -> String {
    let mut used = 0;
    text.chars()
        .take_while(|character| {
            let next = UnicodeWidthChar::width(*character).unwrap_or(0);
            if used + next <= width {
                used += next;
                true
            } else {
                false
            }
        })
        .collect()
}

fn center(text: &str, width: u16) -> String {
    let text = truncate_to_width(text, width as usize);
    let remaining = (width as usize).saturating_sub(display_width(&text));
    let left = remaining / 2;
    format!(
        "{}{}{}",
        " ".repeat(left),
        text,
        " ".repeat(remaining - left)
    )
}

fn art_lines(state: &State, max_width: u16, max_height: usize) -> Vec<String> {
    state
        .ascii_art
        .as_deref()
        .unwrap_or("")
        .lines()
        .take(max_height)
        .map(|line| truncate_to_width(line, max_width as usize))
        .collect()
}

fn progress_line(state: &State, cfg: &Config, width: u16) -> String {
    let total = match state.phase {
        Phase::Work => cfg.work_sec,
        Phase::Short => cfg.short_sec,
        Phase::Long => cfg.long_sec,
        Phase::Idle => 1,
    } as f64;
    let pct = if state.phase == Phase::Idle {
        0.0
    } else {
        ((total - state.remaining) / total).clamp(0.0, 1.0)
    };
    let inner = width.saturating_sub(2).clamp(5, 70) as usize;
    let filled = (pct * inner as f64).round() as usize;
    format!("[{}{}]", "#".repeat(filled), "-".repeat(inner - filled))
}

fn normal_screen(state: &State, cfg: &Config, width: u16, height: u16) -> Vec<String> {
    if width < 18 || height < 9 {
        return [
            "PTIMER".to_string(),
            state.phase.label().to_string(),
            human_mmss(state.remaining),
            "s e q".to_string(),
        ]
        .into_iter()
        .take(height as usize)
        .map(|line| truncate_to_width(&line, width as usize))
        .collect();
    }
    let content_width = width.min(120).saturating_sub(2);
    let mut lines = vec![center("PTIMER", content_width), String::new()];
    let status = if state.paused {
        format!("[ {} / PAUSED ]", state.phase.label())
    } else {
        format!("[ {} ]", state.phase.label())
    };
    if cfg.art_style.is_grand() && width >= 76 && height >= 22 {
        lines.push(center(&status, content_width));
        lines.push(String::new());
        let mut clock = clock_lines(state, cfg, content_width);
        if height < 28 && clock.len() > 1 {
            clock = vec![if state.phase == Phase::Idle {
                "--:--".to_string()
            } else {
                human_mmss(state.remaining)
            }];
        }
        lines.extend(clock.into_iter().map(|line| center(&line, content_width)));
        lines.push(String::new());
        let available_art_height = height.saturating_sub(lines.len() as u16 + 6) as usize;
        lines.extend(
            art_lines(state, content_width, available_art_height)
                .into_iter()
                .map(|line| center(&line, content_width)),
        );
    } else if width >= 68 && height >= 15 {
        let left_width = (content_width / 2).max(28);
        let right_width = content_width.saturating_sub(left_width + 2);
        let mut left = vec![center(&status, left_width)];
        left.push(String::new());
        left.extend(
            clock_lines(state, cfg, left_width)
                .into_iter()
                .map(|line| center(&line, left_width)),
        );
        let art = art_lines(state, right_width, height.saturating_sub(9) as usize);
        let row_count = left.len().max(art.len());
        for row in 0..row_count {
            let lhs = left.get(row).cloned().unwrap_or_default();
            let lhs_padding = left_width as usize - display_width(&lhs).min(left_width as usize);
            let rhs = art.get(row).cloned().unwrap_or_default();
            let rhs_padding = right_width as usize - display_width(&rhs).min(right_width as usize);
            lines.push(format!(
                "{}{}  {}{}",
                lhs,
                " ".repeat(lhs_padding),
                rhs,
                " ".repeat(rhs_padding)
            ));
        }
    } else {
        lines.push(center(&status, content_width));
        lines.push(String::new());
        lines.extend(
            clock_lines(state, cfg, content_width)
                .into_iter()
                .map(|line| center(&line, content_width)),
        );
        if height >= 17 {
            lines.push(String::new());
            lines.extend(
                art_lines(
                    state,
                    content_width,
                    height.saturating_sub(lines.len() as u16 + 7) as usize,
                )
                .into_iter()
                .map(|line| center(&line, content_width)),
            );
        }
    }
    lines.push(String::new());
    lines.push(center(
        &progress_line(state, cfg, content_width),
        content_width,
    ));
    if lines.len() + 3 < height as usize {
        lines.push(String::new());
        lines.push(center(
            &format!(
                "sessions {} / long break every {}",
                state.sessions_completed, cfg.long_every
            ),
            content_width,
        ));
        lines.push(center(
            "[s] start  [p] pause  [e] settings  [q] quit",
            content_width,
        ));
    } else {
        lines.push(center(
            "s:start  p:pause  e:settings  q:quit",
            content_width,
        ));
    }
    lines
}

const SETTING_COUNT: usize = 9;

fn settings_screen(state: &State, cfg: &Config, width: u16, height: u16) -> Vec<String> {
    let content_width = width.min(72).saturating_sub(2).max(1);
    let values = [
        ("Work duration", format!("{} min", cfg.work_sec / 60)),
        ("Short break", format!("{} min", cfg.short_sec / 60)),
        ("Long break", format!("{} min", cfg.long_sec / 60)),
        ("Long break every", format!("{} sessions", cfg.long_every)),
        ("ASCII art", cfg.art_style.label().to_string()),
        (
            "Animate art",
            if cfg.animations_enabled { "on" } else { "off" }.to_string(),
        ),
        ("Clock", cfg.clock_style.label().to_string()),
        ("Desktop notice", cfg.notifications.label().to_string()),
        ("Sound alert", cfg.sound.label().to_string()),
    ];
    let mut lines = vec![
        center("PTIMER SETTINGS", content_width),
        center("use arrows to select and change", content_width),
        String::new(),
    ];
    let reserved_rows = if state.save_message.is_some() { 6 } else { 5 };
    let visible_count = (height as usize)
        .saturating_sub(reserved_rows)
        .max(1)
        .min(values.len());
    let first_visible = state
        .selected_setting
        .saturating_sub(visible_count / 2)
        .min(values.len() - visible_count);
    for (index, (name, value)) in values
        .iter()
        .enumerate()
        .skip(first_visible)
        .take(visible_count)
    {
        let marker = if index == state.selected_setting {
            ">"
        } else {
            " "
        };
        let available = content_width as usize;
        let fixed = display_width(marker) + display_width(value) + 3;
        let name = truncate_to_width(name, available.saturating_sub(fixed));
        let gap = available.saturating_sub(display_width(&name) + display_width(value) + 2);
        lines.push(format!("{} {}{}{}", marker, name, " ".repeat(gap), value));
    }
    lines.push(String::new());
    if let Some(message) = &state.save_message {
        lines.push(center(message, content_width));
    }
    lines.push(center(
        "[t] test alerts  [r] defaults  [enter/e/esc] done",
        content_width,
    ));
    lines.truncate(height as usize);
    lines
}

fn draw(state: &State, cfg: &Config) -> Result<()> {
    let mut out = io::stdout();
    let (cols, rows) = state.last_size;
    queue!(
        out,
        cursor::Hide,
        cursor::MoveTo(0, 0),
        Clear(ClearType::All)
    )?;
    let lines = if state.settings_open {
        settings_screen(state, cfg, cols, rows)
    } else {
        normal_screen(state, cfg, cols, rows)
    };
    let block_height = lines.len().min(rows as usize) as u16;
    let y_offset = rows.saturating_sub(block_height) / 2;
    for (row, line) in lines.iter().take(rows as usize).enumerate() {
        let selected_settings_row = state.settings_open && line.starts_with('>');
        let line = truncate_to_width(line, cols as usize);
        let x = cols.saturating_sub(display_width(&line) as u16) / 2;
        let color = if state.settings_open {
            if selected_settings_row {
                Color::Cyan
            } else {
                Color::Grey
            }
        } else {
            match state.phase {
                Phase::Idle => Color::Yellow,
                Phase::Work => Color::Cyan,
                Phase::Short => Color::Green,
                Phase::Long => Color::Magenta,
            }
        };
        queue!(
            out,
            cursor::MoveTo(x, y_offset + row as u16),
            SetForegroundColor(color)
        )?;
        if row == 0 {
            queue!(out, SetAttribute(Attribute::Bold))?;
        }
        if selected_settings_row {
            queue!(out, SetAttribute(Attribute::Reverse))?;
        }
        queue!(out, Print(line), SetAttribute(Attribute::Reset), ResetColor)?;
    }
    out.flush()?;
    Ok(())
}

fn change_setting(state: &mut State, cfg: &mut Config, delta: i32) {
    let adjust = |value: u64, minimum: u64, maximum: u64, step: u64| -> u64 {
        if delta < 0 {
            value.saturating_sub(step).max(minimum)
        } else {
            value.saturating_add(step).min(maximum)
        }
    };
    match state.selected_setting {
        0 => cfg.work_sec = adjust(cfg.work_sec / 60, 1, 240, 1) * 60,
        1 => cfg.short_sec = adjust(cfg.short_sec / 60, 1, 120, 1) * 60,
        2 => cfg.long_sec = adjust(cfg.long_sec / 60, 1, 180, 1) * 60,
        3 => cfg.long_every = adjust(cfg.long_every, 1, 20, 1),
        4 => {
            cfg.art_style = cfg.art_style.cycle(delta);
            state.animation_frame = 0;
            state.ascii_art = read_ascii_for_phase(state.phase, cfg, 0);
        }
        5 => {
            cfg.animations_enabled = !cfg.animations_enabled;
            state.animation_frame = 0;
            state.ascii_art = read_ascii_for_phase(state.phase, cfg, 0);
        }
        6 => cfg.clock_style = cfg.clock_style.cycle(delta),
        7 => cfg.notifications = cfg.notifications.cycle(delta),
        8 => cfg.sound = cfg.sound.cycle(delta),
        _ => {}
    }
    state.save_message = Some(match cfg.save() {
        Ok(()) => "saved".into(),
        Err(error) => format!("save failed: {error}"),
    });
}

fn reset_settings(state: &mut State, cfg: &mut Config) {
    let defaults = Preferences::default();
    cfg.work_sec = defaults.work_min * 60;
    cfg.short_sec = defaults.short_min * 60;
    cfg.long_sec = defaults.long_min * 60;
    cfg.long_every = defaults.long_every;
    cfg.art_style = defaults.art_style;
    cfg.animations_enabled = defaults.animations_enabled;
    cfg.clock_style = defaults.clock_style;
    cfg.notifications = defaults.notifications;
    cfg.sound = defaults.sound;
    state.animation_frame = 0;
    state.ascii_art = read_ascii_for_phase(state.phase, cfg, 0);
    state.save_message = Some(match cfg.save() {
        Ok(()) => "defaults restored and saved".into(),
        Err(error) => format!("save failed: {error}"),
    });
}

struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, cursor::Show);
    }
}

fn main() -> Result<()> {
    let mut cfg = parse_cli();
    if !cfg.log_path.exists()
        && let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&cfg.log_path)
    {
        let _ = writeln!(file, "timestamp,phase,duration_sec,notes");
    }
    let mut state = State {
        phase: Phase::Idle,
        remaining: 0.0,
        started_at: None,
        paused: false,
        sessions_completed: 0,
        ascii_art: read_ascii_for_phase(Phase::Idle, &cfg, 0),
        last_size: terminal::size()?,
        settings_open: false,
        selected_setting: 0,
        save_message: None,
        animation_frame: 0,
    };
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        cursor::Hide,
        SetTitle("ptimer")
    )?;
    terminal::enable_raw_mode()?;
    let _guard = TerminalGuard;
    draw(&state, &cfg)?;
    let tick = Duration::from_millis(100);
    let mut previous_tick = Instant::now();
    let mut last_drawn_second = -1;
    let mut last_animation_draw = Instant::now();
    'outer: loop {
        if event::poll(tick)? {
            match event::read()? {
                Event::Key(KeyEvent {
                    code,
                    modifiers,
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                }) => {
                    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
                        break 'outer;
                    }
                    if state.settings_open {
                        match code {
                            KeyCode::Up => {
                                state.selected_setting = state
                                    .selected_setting
                                    .checked_sub(1)
                                    .unwrap_or(SETTING_COUNT - 1)
                            }
                            KeyCode::Down => {
                                state.selected_setting =
                                    (state.selected_setting + 1) % SETTING_COUNT
                            }
                            KeyCode::Left => change_setting(&mut state, &mut cfg, -1),
                            KeyCode::Right => change_setting(&mut state, &mut cfg, 1),
                            KeyCode::Char('r') => reset_settings(&mut state, &mut cfg),
                            KeyCode::Char('t') => {
                                test_alert(&cfg);
                                state.save_message = Some("test alert sent".into());
                            }
                            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('e') => {
                                state.settings_open = false
                            }
                            KeyCode::Char('q') => break 'outer,
                            _ => {}
                        }
                        draw(&state, &cfg)?;
                        continue;
                    }
                    match code {
                        KeyCode::Char('q') | KeyCode::Esc => break 'outer,
                        KeyCode::Char('e') => {
                            state.settings_open = true;
                            state.save_message = None;
                            draw(&state, &cfg)?;
                        }
                        KeyCode::Char('s')
                            if matches!(state.phase, Phase::Idle | Phase::Short | Phase::Long) =>
                        {
                            update_phase(&mut state, Phase::Work, &cfg);
                            state.remaining = cfg.work_sec as f64;
                            state.started_at = Some(Instant::now());
                            state.paused = false;
                            draw(&state, &cfg)?;
                        }
                        KeyCode::Char('p') if state.phase != Phase::Idle => {
                            state.paused = !state.paused;
                            draw(&state, &cfg)?;
                        }
                        _ => {}
                    }
                }
                Event::Resize(cols, rows) => {
                    state.last_size = (cols, rows);
                    draw(&state, &cfg)?;
                }
                _ => {}
            }
        }
        let now = Instant::now();
        let elapsed = now.duration_since(previous_tick).as_secs_f64();
        previous_tick = now;
        let mut drew_this_tick = false;
        if !state.paused
            && !state.settings_open
            && matches!(state.phase, Phase::Work | Phase::Short | Phase::Long)
        {
            state.remaining -= elapsed;
            if state.remaining <= 0.0 {
                let finished = state.phase;
                let seconds = match finished {
                    Phase::Work => cfg.work_sec,
                    Phase::Short => cfg.short_sec,
                    Phase::Long => cfg.long_sec,
                    Phase::Idle => 0,
                };
                log_phase(&cfg.log_path, finished, seconds, state.started_at);
                alert_user(finished, &cfg);
                if finished == Phase::Work {
                    state.sessions_completed += 1;
                    let long = state.sessions_completed.is_multiple_of(cfg.long_every);
                    update_phase(
                        &mut state,
                        if long { Phase::Long } else { Phase::Short },
                        &cfg,
                    );
                    state.remaining = if long { cfg.long_sec } else { cfg.short_sec } as f64;
                    state.started_at = Some(Instant::now());
                } else {
                    update_phase(&mut state, Phase::Idle, &cfg);
                    state.remaining = 0.0;
                    state.started_at = None;
                }
                last_drawn_second = -1;
                draw(&state, &cfg)?;
                drew_this_tick = true;
            } else {
                let second = state.remaining.ceil() as i64;
                if second != last_drawn_second {
                    last_drawn_second = second;
                    draw(&state, &cfg)?;
                    drew_this_tick = true;
                }
            }
        }
        if !state.settings_open
            && cfg.animations_enabled
            && let Some(interval) = cfg.art_style.frame_interval()
            && now.duration_since(last_animation_draw) >= interval
        {
            state.animation_frame = state.animation_frame.wrapping_add(1);
            state.ascii_art = read_ascii_for_phase(state.phase, &cfg, state.animation_frame);
            last_animation_draw = now;
            if !drew_this_tick {
                draw(&state, &cfg)?;
            }
        }
    }
    if matches!(state.phase, Phase::Work | Phase::Short | Phase::Long) {
        let seconds = match state.phase {
            Phase::Work => cfg.work_sec,
            Phase::Short => cfg.short_sec,
            Phase::Long => cfg.long_sec,
            Phase::Idle => 0,
        };
        log_phase(&cfg.log_path, state.phase, seconds, state.started_at);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_respects_wide_characters() {
        assert_eq!(truncate_to_width("ab界cd", 4), "ab界");
        assert_eq!(display_width(&truncate_to_width("ab界cd", 4)), 4);
    }

    #[test]
    fn cycling_wraps_in_both_directions() {
        assert_eq!(ArtStyle::Custom.cycle(-1), ArtStyle::Off);
        assert_eq!(ClockStyle::Minimal.cycle(1), ClockStyle::Digital);
    }

    #[test]
    fn countdown_rounds_up_for_display() {
        assert_eq!(human_mmss(59.1), "01:00");
        assert_eq!(human_mmss(59.0), "00:59");
    }

    #[test]
    fn older_preferences_keep_animation_enabled() {
        let preferences: Preferences = toml::from_str(
            r#"
work_min = 25
short_min = 5
long_min = 15
long_every = 4
art_style = "custom"
clock_style = "digital"
"#,
        )
        .unwrap();
        assert!(preferences.animations_enabled);
        assert_eq!(preferences.notifications, NotificationStyle::Urgent);
        assert_eq!(preferences.sound, SoundStyle::Bell);
    }

    fn test_config() -> Config {
        Config {
            work_sec: 25 * 60,
            short_sec: 5 * 60,
            long_sec: 15 * 60,
            long_every: 4,
            ascii_dir: PathBuf::new(),
            ascii_config: default_toml_config().ascii_art,
            log_path: PathBuf::new(),
            preferences_path: PathBuf::new(),
            art_style: ArtStyle::Tomato,
            animations_enabled: true,
            clock_style: ClockStyle::Digital,
            notifications: NotificationStyle::Urgent,
            sound: SoundStyle::Bell,
        }
    }

    fn test_state(size: (u16, u16)) -> State {
        State {
            phase: Phase::Work,
            remaining: 1499.2,
            started_at: None,
            paused: false,
            sessions_completed: 2,
            ascii_art: Some(animation::tomato(0)),
            last_size: size,
            settings_open: false,
            selected_setting: 0,
            save_message: None,
            animation_frame: 0,
        }
    }

    #[test]
    fn responsive_screens_fit_common_terminal_sizes() {
        let cfg = test_config();
        for (width, height) in [(120, 35), (70, 20), (45, 16), (20, 7)] {
            let state = test_state((width, height));
            let lines = normal_screen(&state, &cfg, width, height);
            assert!(lines.len() <= height as usize);
            assert!(
                lines
                    .iter()
                    .all(|line| display_width(line) <= width as usize)
            );

            let lines = settings_screen(&state, &cfg, width, height);
            assert!(lines.len() <= height as usize);
            assert!(
                lines
                    .iter()
                    .all(|line| display_width(line) <= width as usize)
            );

            let mut state = state;
            state.selected_setting = SETTING_COUNT - 1;
            let lines = settings_screen(&state, &cfg, width, height);
            assert!(lines.iter().any(|line| line.starts_with('>')));
        }
    }

    #[test]
    fn original_cat_is_the_pre_animation_bundled_art() {
        let mut cfg = test_config();
        cfg.art_style = ArtStyle::OriginalCat;
        assert_eq!(
            read_ascii_for_phase(Phase::Idle, &cfg, 99).as_deref(),
            Some(DEFAULT_ASCII_ART.trim_end_matches('\n'))
        );
    }
}
