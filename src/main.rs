use chrono::Local;
use console::{pad_str, Alignment};
use terminal_size::{terminal_size, Width, Height};
use std::io::Write;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use crossterm::event::{poll, read, Event, KeyCode, KeyModifiers};
use std::time::Duration;

const DIGITS: [[&str; 11]; 5] = [
    [" ▓▓▓▓▓▓ ", " ▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓  ▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", "    "],
    [" ▓▓  ▓▓ ", "  ▓▓ ", "     ▓▓ ", "     ▓▓ ", " ▓▓  ▓▓ ", " ▓▓     ", " ▓▓     ", "     ▓▓ ", " ▓▓  ▓▓ ", " ▓▓  ▓▓ ", " ▓▓ "],
    [" ▓▓  ▓▓ ", "  ▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", "     ▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", "    "],
    [" ▓▓  ▓▓ ", "  ▓▓ ", " ▓▓     ", "     ▓▓ ", "     ▓▓ ", "     ▓▓ ", " ▓▓  ▓▓ ", "     ▓▓ ", " ▓▓  ▓▓ ", "     ▓▓ ", " ▓▓ "],
    [" ▓▓▓▓▓▓ ", "  ▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", "     ▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", "     ▓▓ ", " ▓▓▓▓▓▓ ", " ▓▓▓▓▓▓ ", "    "],
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
    print!("\x1b[37m");

    'outer: loop {
        let t = Local::now();
        let time = t.format("%H:%M:%S").to_string();
        let (width, height) = term_size();

        let clock_h = DIGITS.len();
        let total_h = clock_h + 2;
        let top = height.saturating_sub(total_h) / 2;

        print!("\x1b[2J\x1b[H");

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

        let date = t.format("%Y - %m - %d").to_string();
        print!("\x1b[{};1H{}", top + clock_h + 2, pad_str(&date, width, Alignment::Center, None));

        std::io::stdout().flush().unwrap();

        if poll(Duration::from_millis(1000)).unwrap() {
            if let Event::Key(k) = read().unwrap() {
                if k.code == KeyCode::Char('c')
                    && k.modifiers.contains(KeyModifiers::CONTROL)
                {
                    break 'outer;
                }
            }
        }
    }

    disable_raw_mode().unwrap();
    print!("\x1b[?25h");
    print!("\x1b[?1049l");
}
