use chrono::Local;
use console::{pad_str, Alignment};
use terminal_size::{terminal_size, Width, Height};
use std::io::Write;
use std::time::Instant;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use crossterm::event::{poll, read, Event, KeyCode, KeyModifiers};
use std::time::Duration;

const DIGITS: [[&str; 11]; 5] = [
    [" ▓▓▓▓▓▓ ", "    ▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓  ▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", "    "],
    [" ▓▓  ▓▓ ", "     ▓▓ ", "     ▓▓ ", "     ▓▓ ", " ▓▓  ▓▓ ", " ▓▓     ", " ▓▓     ", "     ▓▓ ", " ▓▓  ▓▓ ", " ▓▓  ▓▓ ", " ▓▓ "],
    [" ▓▓  ▓▓ ", "     ▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", "     ▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", "    "],
    [" ▓▓  ▓▓ ", "     ▓▓ ", " ▓▓     ", "     ▓▓ ", "     ▓▓ ", "     ▓▓ ", " ▓▓  ▓▓ ", "     ▓▓ ", " ▓▓  ▓▓ ", "     ▓▓ ", " ▓▓ "],
    [" ▓▓▓▓▓▓ ", "     ▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", "     ▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", "     ▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", "    "],
];

fn term_size() -> (usize, usize) {
    terminal_size()
        .map(|(Width(w), Height(h))| (w as usize, h as usize))
        .unwrap_or((80, 24))
}

fn main() {
    enable_raw_mode().unwrap();

    print!("\x1b[?1049h");
    print!("\x1b[2J");
    print!("\x1b[?25l");

    let mut show_seconds = false;
    let mut marks: Vec<String> = Vec::new();
    let mut color: u8 = 37;
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "-c") {
        if let Some(val) = args.get(pos + 1) {
            if let Ok(n) = val.parse::<u8>() {
                if (1..=8).contains(&n) {
                    color = 30 + (n - 1);
                }
            }
        }
    }
    if args.iter().any(|a| a == "-s") {
        show_seconds = true;
    }

    let mut chrono_mode = false;
    let mut chrono_running = false;
    let mut chrono_elapsed = Duration::ZERO;
    let mut chrono_start = Instant::now();

    'outer: loop {
        let t = Local::now();

        let time = if chrono_mode {
            let total = if chrono_running {
                chrono_elapsed + chrono_start.elapsed()
            } else {
                chrono_elapsed
            };
            let secs = total.as_secs();
            let ms = total.subsec_millis();
            format!("{:02}:{:02}:{:03}", secs / 60, secs % 60, ms)
        } else if show_seconds {
            t.format("%H:%M:%S").to_string()
        } else {
            t.format("%H:%M").to_string()
        };

        let (width, height) = term_size();

        let clock_h = DIGITS.len();
        let block_h = clock_h + 2;
        let top = height.saturating_sub(block_h) / 2;
        let active_color = if chrono_mode {
            if chrono_running { 32 } else { 33 }
        } else {
            color
        };
        print!("\x1b[2J\x1b[H\x1b[{}m", active_color);

        for (i, row) in DIGITS.iter().enumerate() {
            let mut line = String::new();
            for c in time.chars() {
                let col = match c {
                    '0'..='9' => c as usize - '0' as usize,
                    _ => 10,
                };
                line.push_str(row[col]);
            }
            print!("\x1b[{};1H{}", top + i + 1, pad_str(&line, width, Alignment::Center, None));
        }

        let bottom = if chrono_mode {
            if chrono_running { " Running" } else { " Stopped" }.to_string()
        } else {
            t.format("%Y - %m - %d").to_string()
        };
        print!("\x1b[{};1H{}", top + clock_h + 2, pad_str(&bottom, width, Alignment::Center, None));

        if chrono_mode {
            for (i, m) in marks.iter().enumerate() {
                print!("\x1b[{};1H{}", top + clock_h + 3 + i, pad_str(m, width, Alignment::Center, None));
            }
        }

        std::io::stdout().flush().unwrap();

        let tick = if chrono_mode && chrono_running { 16 } else { 1000 };
        if poll(Duration::from_millis(tick)).unwrap() {
            if let Event::Key(k) = read().unwrap() {
                if k.code == KeyCode::Char('c')
                    && k.modifiers.contains(KeyModifiers::CONTROL)
                {
                    break 'outer;
                }
                if k.code == KeyCode::Enter && chrono_mode && chrono_running {
                    let total = chrono_elapsed + chrono_start.elapsed();
                    let secs = total.as_secs();
                    marks.insert(0, format!("{:02}:{:02}:{:03}", secs / 60, secs % 60, total.subsec_millis()));
                    if marks.len() > 5 {
                        marks.pop();
                    }
                }
                if let KeyCode::Char(c) = k.code {
                    match c {
                        '1'..='8' => color = 30 + (c as u8 - b'1'),
                        's' if !chrono_mode => show_seconds = !show_seconds,
                        'c' => {
                            chrono_mode = !chrono_mode;
                            if chrono_mode {
                                chrono_elapsed = Duration::ZERO;
                                chrono_running = false;
                                marks.clear();
                            }
                        }
                        ' ' if chrono_mode => {
                            if chrono_running {
                                chrono_elapsed += chrono_start.elapsed();
                                chrono_running = false;
                            } else {
                                chrono_start = Instant::now();
                                chrono_running = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode().unwrap();
    print!("\x1b[?25h");
    print!("\x1b[?1049l");
}
