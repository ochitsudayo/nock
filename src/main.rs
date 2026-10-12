use chrono::Local;
use console::{pad_str, Alignment};
use crossterm::event::{poll, read, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::Write;
use std::time::{Duration, Instant};
use terminal_size::{terminal_size, Height, Width};

const DIGITS: [[&str; 11]; 5] = [
    [" ██████ ", " ███ ", " ██████ ", " ██████ ", " ██  ██ ", " ██████ ", " ██████ ", " ██████ ", " ██████ ", " ██████ ", "    "],
    [" ██  ██ ", "  ██ ", "     ██ ", "     ██ ", " ██  ██ ", " ██     ", " ██     ", "     ██ ", " ██  ██ ", " ██  ██ ", " ██ "],
    [" ██  ██ ", "  ██ ", " ██████ ", " ██████ ", " ██████ ", " ██████ ", " ██████ ", "     ██ ", " ██████ ", " ██████ ", "    "],
    [" ██  ██ ", "  ██ ", " ██     ", "     ██ ", "     ██ ", "     ██ ", " ██  ██ ", "     ██ ", " ██  ██ ", "     ██ ", " ██ "],
    [" ██████ ", "  ██ ", " ██████ ", " ██████ ", "     ██ ", " ██████ ", " ██████ ", "     ██ ", " ██████ ", " ██████ ", "    "],
];

const BLANK_COLUMN: usize = 10;
const FONT_ROWS: usize = DIGITS.len();

fn column_for(c: char) -> usize {
    match c {
        '0'..='9' => c as usize - '0' as usize,
        _ => BLANK_COLUMN,
    }
}

const COLOR_YELLOW: u8 = 33;
const COLOR_YELLOW_BRIGHT: u8 = 93;
const COLOR_WHITE: u8 = 37;
const COLOR_MAGENTA: u8 = 35;
const COLOR_MAGENTA_BRIGHT: u8 = 95;
const COLOR_MIN: u8 = 1;
const COLOR_MAX: u8 = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Clock,
    Chrono,
    Timer,
}

struct Nock {
    mode: Mode,
    color: u8,
    show_seconds: bool,

    chronometer_running: bool,
    chronometer_elapsed: Duration,
    chronometer_started: Instant,
    laps: Vec<String>,

    timer_fields: [u8; 3],
    timer_cursor: usize,
    timer_running: bool,
    timer_remaining: Duration,
    timer_started: Instant,
}

impl Nock {
    fn new() -> Self {
        Self {
            mode: Mode::Clock,
            color: COLOR_WHITE,
            show_seconds: false,
            chronometer_running: false,
            chronometer_elapsed: Duration::ZERO,
            chronometer_started: Instant::now(),
            laps: Vec::new(),
            timer_fields: [0, 0, 0],
            timer_cursor: 0,
            timer_running: false,
            timer_remaining: Duration::ZERO,
            timer_started: Instant::now(),
        }
    }

    // chronometer
    fn chronometer_total(&self) -> Duration {
        if self.chronometer_running {
            self.chronometer_elapsed + self.chronometer_started.elapsed()
        } else {
            self.chronometer_elapsed
        }
    }

    fn enter_chronometer(&mut self) {
        self.mode = Mode::Chrono;
        self.chronometer_elapsed = Duration::ZERO;
        self.chronometer_running = false;
        self.laps.clear();
    }

    fn chronometer_toggle(&mut self) {
        if self.chronometer_running {
            self.chronometer_elapsed += self.chronometer_started.elapsed();
            self.chronometer_running = false;
        } else {
            self.chronometer_started = Instant::now();
            self.chronometer_running = true;
        }
    }

    fn chronometer_laps(&mut self) {
        let total = self.chronometer_total();
        let secs = total.as_secs();
        self.laps.insert(0, format!("{:02}:{:02}:{:03}", secs / 60, secs % 60, total.subsec_millis()));
        if self.laps.len() > 5 {
            self.laps.pop();
        }
    }

    // timer
    fn enter_timer_mode(&mut self) {
        self.mode = Mode::Timer;
        self.timer_running = false;
    }

    fn timer_toggle(&mut self) {
        if self.timer_running {
            self.timer_remaining = self.timer_remaining.saturating_sub(self.timer_started.elapsed());
            self.timer_running = false;
        } else {
            let secs = self.timer_fields[0] as u64 * 3600
                + self.timer_fields[1] as u64 * 60
                + self.timer_fields[2] as u64;
            if secs > 0 {
                self.timer_remaining = Duration::from_secs(secs);
                self.timer_started = Instant::now();
                self.timer_running = true;
            }
        }
    }

    fn timer_display_remaining(&self) -> Duration {
        if self.timer_running {
            self.timer_remaining.saturating_sub(self.timer_started.elapsed())
        } else {
            self.timer_remaining
        }
    }

    fn timer_clear(&mut self) {
        self.timer_fields = [0, 0, 0];
        self.timer_cursor = 0;
        self.timer_running = false;
        self.timer_remaining = Duration::ZERO;
    }

    fn timer_adjust(&mut self, delta: i8) {
        let field = &mut self.timer_fields[self.timer_cursor];
        *field = if delta >= 0 {
            (*field + delta as u8) % 100
        } else {
            (*field + 100 - (-delta) as u8) % 100
        };
    }

    fn timer_move_cursor(&mut self, delta: i8) {
        let next = self.timer_cursor as i8 + delta;
        self.timer_cursor = next.rem_euclid(3) as usize;
    }

    fn timer_tick(&mut self) {
        if self.mode == Mode::Timer
            && self.timer_running
            && self.timer_started.elapsed() >= self.timer_remaining
        {
            self.timer_running = false;
            self.timer_remaining = Duration::ZERO;
            std::process::Command::new("notify-send")
                .args(["Nock", "Time's over"])
                .spawn()
                .ok();
        }
    }
}

fn timer_highlight(nock: &Nock) -> Option<(usize, usize)> {
    if nock.mode == Mode::Timer && !nock.timer_running {
        let start = nock.timer_cursor * 3;
        Some((start, start + 1))
    } else {
        None
    }
}

// terminal and application
fn term_size() -> (usize, usize) {
    terminal_size()
        .map(|(Width(w), Height(h))| (w as usize, h as usize))
        .unwrap_or((80, 24))
}

fn nock_display(nock: &Nock) -> String {
    match nock.mode {
        Mode::Chrono => {
            let total = nock.chronometer_total();
            let secs = total.as_secs();
            format!("{:02}:{:02}:{:03}", secs / 60, secs % 60, total.subsec_millis())
        }
        Mode::Timer => {
            if nock.timer_running {
                let secs = nock.timer_display_remaining().as_secs();
                format!("{:02}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60)
            } else {
                format!("{:02}:{:02}:{:02}", nock.timer_fields[0], nock.timer_fields[1], nock.timer_fields[2])
            }
        }
        Mode::Clock => {
            let now = Local::now();
            if nock.show_seconds {
                now.format("%H:%M:%S").to_string()
            } else {
                now.format("%H:%M").to_string()
            }
        }
    }
}

fn clock_undertext(nock: &Nock) -> String {
    match nock.mode {
        Mode::Chrono => running_text(nock.chronometer_running),
        Mode::Timer => running_text(nock.timer_running),
        Mode::Clock => Local::now().format("%Y - %m - %d").to_string(),
    }
}

fn running_text(running: bool) -> String {
    if running { " Running" } else { " Stopped" }.to_string()
}

fn clock_color(nock: &Nock) -> u8 {
    match nock.mode {
        Mode::Chrono => if nock.chronometer_running { COLOR_YELLOW_BRIGHT } else { COLOR_YELLOW },
        Mode::Timer => if nock.timer_running { COLOR_MAGENTA_BRIGHT } else { COLOR_MAGENTA },
        Mode::Clock => nock.color,
    }
}

fn render_row(
    row: &[&str; 11],
    text: &[char],
    highlight: Option<(usize, usize)>,
    base_color: u8,
) -> String {
    let mut line = String::new();
    for (i, &c) in text.iter().enumerate() {
        let cell = row[column_for(c)];
        let highlighted = highlight
            .map(|(lo, hi)| i >= lo && i <= hi)
            .unwrap_or(false);
        if highlighted {
            line.push_str(&format!("\x1b[{}m{}", COLOR_MAGENTA_BRIGHT, cell));
            line.push_str(&format!("\x1b[{}m", base_color));
        } else {
            line.push_str(cell);
        }
    }
    line
}

fn draw_frame(nock: &Nock) {
    let (width, height) = term_size();
    let text: Vec<char> = nock_display(nock).chars().collect();

    let block_h = FONT_ROWS + 2;
    let top = height.saturating_sub(block_h) / 2;

    let base_color = clock_color(nock);
    let highlight = timer_highlight(nock);

    print!("\x1b[2J\x1b[H\x1b[{}m", base_color);

    for (i, row) in DIGITS.iter().enumerate() {
        let line = render_row(row, &text, highlight, base_color);
        let screen_row = top + i + 1;
        print!(
            "\x1b[{};1H{}",
            screen_row,
            pad_str(&line, width, Alignment::Center, None)
        );
    }

    let status_row = top + FONT_ROWS + 2;
    print!(
        "\x1b[{};1H{}",
        status_row,
        pad_str(&clock_undertext(nock), width, Alignment::Center, None)
    );

    if nock.mode == Mode::Chrono {
        for (i, lap) in nock.laps.iter().enumerate() {
            let row = status_row + 1 + i;
            print!(
                "\x1b[{};1H{}",
                row,
                pad_str(lap, width, Alignment::Center, None)
            );
        }
    }

    std::io::stdout().flush().unwrap();
}

fn redraw_interval(nock: &Nock) -> Duration {
    let animating = (nock.mode == Mode::Chrono && nock.chronometer_running)
        || (nock.mode == Mode::Timer && nock.timer_running);
    if animating {
        Duration::from_millis(16)
    } else {
        Duration::from_millis(1000)
    }
}

fn handle_keypress(nock: &mut Nock, key: crossterm::event::KeyEvent) -> bool {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return true;
    }

    if key.code == KeyCode::Char('C') && key.modifiers.contains(KeyModifiers::SHIFT) {
        nock.enter_chronometer();
        return false;
    }

    if let KeyCode::Char(c) = key.code {
        match c {
            '1'..='8' => {
                nock.color = 30 + (c as u8 - b'1');
            }
            's' if nock.mode == Mode::Clock => nock.show_seconds = !nock.show_seconds,
            'c' => nock.mode = Mode::Clock,
            't' => nock.enter_timer_mode(),
            _ => {}
        }
    }

    match nock.mode {
        Mode::Clock => {}
        Mode::Chrono => handle_chronometer_key(nock, key.code),
        Mode::Timer => handle_timer_key(nock, key.code),
    }

    false
}

fn handle_chronometer_key(nock: &mut Nock, code: KeyCode) {
    match code {
        KeyCode::Enter if nock.chronometer_running => nock.chronometer_laps(),
        KeyCode::Char(' ') => nock.chronometer_toggle(),
        _ => {}
    }
}

fn handle_timer_key(nock: &mut Nock, code: KeyCode) {
    if !nock.timer_running {
        match code {
            KeyCode::Char('k') | KeyCode::Up => nock.timer_adjust(1),
            KeyCode::Char('j') | KeyCode::Down => nock.timer_adjust(-1),
            KeyCode::Char('h') | KeyCode::Left => nock.timer_move_cursor(-1),
            KeyCode::Char('l') | KeyCode::Right => nock.timer_move_cursor(1),
            _ => {}
        }
    }
    match code {
        KeyCode::Char(' ') => nock.timer_toggle(),
        KeyCode::Backspace => nock.timer_clear(),
        _ => {}
    }
}

fn cli_flags(nock: &mut Nock) {
    let args: Vec<String> = std::env::args().collect();
    let mut iter = args.iter().skip(1);

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-c" => {
                let val = iter.next();
                let parsed = val.and_then(|v| v.parse::<u8>().ok());
                match parsed {
                    Some(n) if (COLOR_MIN..=COLOR_MAX).contains(&n) => {
                        nock.color = 30 + (n - 1);
                    }
                    _ => {
                        eprintln!(
                            "nock: -c requires a number between {} and {}",
                            COLOR_MIN, COLOR_MAX
                        );
                        eprintln!("Try 'nock -h' for more information.");
                        std::process::exit(1);
                    }
                }
            }"-s" => nock.show_seconds = true,
            "-t" => nock.enter_timer_mode(),
            "-C" => nock.enter_chronometer(),
            "-h" => {
                disable_raw_mode().ok();
                print!("\x1b[?25h\x1b[?1049l");
                println!(concat!(
                        "nock\n",
                        "\n",
                        "USAGE\n",
                        "    nock [FLAGS]\n",
                        "    -c <1-8>    use a terminal color from 0-7\n",
                        "    -s          show seconds in clock mode\n",
                        "    -t          start in timer mode\n",
                        "    -C          start in chronometer mode\n",
                        "    -h          show this message",
                ));
                std::process::exit(0);
            }
            _ => {
                eprintln!("nock: unknown argument '{}'", arg);
                eprintln!("Try 'nock -h' for more information.");
                std::process::exit(1);
            }
        }
    }
}

fn main() {
    let mut nock = Nock::new();
    cli_flags(&mut nock);

    enable_raw_mode().unwrap();
    print!("\x1b[?1049h\x1b[2J\x1b[?25l");

    loop {
        nock.timer_tick();
        draw_frame(&nock);

        if poll(redraw_interval(&nock)).unwrap() {
            if let Event::Key(key) = read().unwrap() {
                if handle_keypress(&mut nock, key) {
                    break;
                }
            }
        }
    }

    disable_raw_mode().unwrap();
    print!("\x1b[?25h\x1b[?1049l");
}
