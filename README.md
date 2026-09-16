# Ptimer - Terminal Pomodoro Timer

A beautiful, minimalist Pomodoro timer for your terminal with ASCII art support and session logging.

## Features

- 🍅 **Pomodoro Technique** - Structured work sessions with breaks
- 🎨 **ASCII Art Display** - Dynamic ASCII art that changes with each phase
- 🖼️ **Large Art Gallery** - Sixteen compact, classic, and grand animated scenes
- 🕐 **Multiple Clocks** - Digital, classic, and minimal timer styles
- 🔔 **Optional Alerts** - Configurable desktop urgency and audible completion sounds
- 📊 **Session Logging** - Automatic CSV logging of all completed sessions
- ⚙️ **Interactive Settings** - Change and save preferences without leaving the app
- 📐 **Responsive Layout** - Adapts from large side-by-side views to compact terminals
- 🎯 **Focused Interface** - Clean, distraction-free terminal UI
- ⏸️ **Pause/Resume** - Full control over your sessions

## Installation

```bash
# Clone the repository
git clone <your-repo-url>
cd ptimer

# Build with cargo
cargo build --release

# Run
cargo run --release
```

## Usage

### Basic Commands

- **`s`** - Start a work session
- **`p`** - Pause/resume the current timer
- **`e`** - Open settings
- **`q`** or **`Esc`** - Quit the application

### Interactive Settings

Press `e` from the timer to open settings. Use:

- **`↑` / `↓`** - Select a setting
- **`←` / `→`** - Change the selected value (hold for quick repeat)
- **`Enter`**, **`e`**, or **`Esc`** - Return to the timer
- **`r`** - Restore defaults
- **`t`** - Test the currently selected desktop and sound alerts

Work, break, art, animation, clock, and notification preferences save immediately to
`~/.config/ptimer/settings.toml` (or the platform-equivalent config directory) and
are restored on the next launch. Command-line options still override saved duration
values for the current run.

### Pomodoro Flow

1. Press `s` to start a 25-minute work session
2. Focus on your task until the timer completes
3. Take a 5-minute short break (automatically starts)
4. After 4 work sessions, enjoy a 15-minute long break
5. Repeat!

## Configuration

Edit `config.toml` in the project root to customize your timer:

```toml
[durations]
work_min = 25      # Work session duration
short_min = 5      # Short break duration
long_min = 15      # Long break duration
long_every = 4     # Number of work sessions before long break

[paths]
log_dir = ".ptimer/logs"          # Log file directory
ascii_dir = ".ptimer/ascii_art"   # ASCII art directory
log_filename = "pomodoro.csv"     # Log file name

[ascii_art]
enabled = true              # Enable/disable ASCII art
idle_file = "idle.txt"      # ASCII art for idle phase
work_file = "work.txt"      # ASCII art for work phase
short_file = "short.txt"    # ASCII art for short break
long_file = "long.txt"      # ASCII art for long break
```

### Command Line Options

Override config settings via command line:

```bash
# Custom durations
ptimer --work 30 --short 10 --long 20

# Custom cycle
ptimer --long-every 3

# Custom paths
ptimer --log /path/to/logs --ascii /path/to/ascii
```

## ASCII Art

ASCII art files are stored in `~/.ptimer/ascii_art/` by default. Each phase has its own file:

- `idle.txt` - Displayed when waiting to start
- `work.txt` - Displayed during work sessions
- `short.txt` - Displayed during short breaks
- `long.txt` - Displayed during long breaks

You can customize these files with your own ASCII art! The art will automatically display alongside the timer.

The settings screen offers sixteen art choices:

- **custom** — your phase-specific files from `~/.ptimer/ascii_art/`
- **original cat** — the exact large Braille cat bundled with the original ptimer
- **tomato**, **coffee**, **small cat**, **focus orb**, and **drifting sky**
- **growing plant**, **hourglass**, **fireplace**, **aquarium**, and **rocket**
- **GRAND cat**, **GRAND castle**, and **GRAND cosmos** — expansive scenes that use
  the full canvas in large terminal windows
- **off** — a clock-only display

The separate **Animate art** setting turns motion on or off without changing the
selected artwork. With animation off, animated scenes remain visible on their first
frame. With it on, they keep moving while the timer is paused. Artwork automatically
clips or hides in small terminal layouts.

### Notifications and sound

The **Desktop notice** setting supports **off**, **normal**, and **urgent**. The
independent **Sound alert** setting supports **off**, **terminal bell**, and
**system chime**. The system chime uses the desktop's standard completion sound and
falls back to the terminal bell if the sound service is unavailable.

Small MIT-licensed animation elements from `cli-spinners` and `asciicity` are
credited in [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md); the complete scenes
around them are original to ptimer.

**Default ASCII Art**: If any ASCII art file is missing, the program will automatically use the default ASCII art included in the project (`default_ascii.txt`). This ensures the interface always looks good even on first run.

## Session Logging

All completed sessions are logged to `~/.ptimer/logs/pomodoro.csv` with:

- Timestamp
- Phase (WORK, PAUSE, LONG PAUSE)
- Duration in seconds
- Status notes

Example log entry:
```csv
timestamp,phase,duration_sec,notes
2025-10-08T14:30:00,WORK,1500,completed
2025-10-08T15:00:00,PAUSE,300,completed
```

## Layout

The interface selects a layout from the available terminal dimensions. Wide windows
place the clock and art side by side, medium windows stack them, and small windows
hide optional details while keeping the timer and controls visible. Resizing is live.

A typical wide layout looks like:

```
              PTIMER

[ WORK ]                        _____ _             
                               / ____| |            
  25:00                       | (___ | |_ __ _ _ __ 
                               \___ \| __/ _` | '__|
                               ____) | || (_| | |   
                              |_____/ \__\__,_|_|   

[############################################################]

sessions: 2  (long every 4)
keys: [s]tart work  [p]ause/resume  [q]uit
log: /home/user/.ptimer/logs/pomodoro.csv
ascii: /home/user/.ptimer/ascii_art
```

## Technical Details

- Built with Rust 🦀
- Uses `crossterm` for terminal rendering
- `serde` and `toml` for configuration
- `chrono` for timestamps
- Unicode display-width-aware responsive layout

## Tips

- Keep the terminal window visible during work sessions
- Customize ASCII art to match your workflow or mood
- Review your session logs to track productivity
- Adjust timings to fit your personal rhythm

## License

[Add your license here]

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.
