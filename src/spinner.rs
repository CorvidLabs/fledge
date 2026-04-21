use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

struct Theme {
    frames: &'static [&'static str],
    interval_ms: u64,
}

const THEMES: &[Theme] = &[
    Theme {
        frames: &[
            "🕐", "🕑", "🕒", "🕓", "🕔", "🕕", "🕖", "🕗", "🕘", "🕙", "🕚", "🕛", " ",
        ],
        interval_ms: 100,
    },
    Theme {
        frames: &["🪨 ", "📄 ", "✂️ ", " "],
        interval_ms: 300,
    },
    Theme {
        frames: &["🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘", " "],
        interval_ms: 150,
    },
    Theme {
        frames: &["☀️ ", "🌤️", "⛅", "🌥️", "☁️ ", "🌧️", "⛈️ ", "🌩️", " "],
        interval_ms: 200,
    },
    Theme {
        frames: &["🌍", "🌎", "🌏", " "],
        interval_ms: 250,
    },
    Theme {
        frames: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", " "],
        interval_ms: 80,
    },
    Theme {
        frames: &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷", " "],
        interval_ms: 80,
    },
    Theme {
        frames: &["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈", " "],
        interval_ms: 100,
    },
    Theme {
        frames: &[
            "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█", "▉", "▊", "▋", "▌", "▍", "▎", "▏", " ",
        ],
        interval_ms: 80,
    },
    Theme {
        frames: &["←", "↖", "↑", "↗", "→", "↘", "↓", "↙", " "],
        interval_ms: 100,
    },
];

pub struct Spinner {
    bar: ProgressBar,
}

impl Spinner {
    pub fn start(message: &str) -> Self {
        let theme = &THEMES[random_index(THEMES.len())];

        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(theme.frames)
                .template("  {msg} {spinner}")
                .expect("valid spinner template"),
        );
        bar.set_message(message.to_string());
        bar.enable_steady_tick(Duration::from_millis(theme.interval_ms));

        Self { bar }
    }

    pub fn finish(&self) {
        self.bar.finish_and_clear();
    }
}

fn random_index(max: usize) -> usize {
    let mut buf = [0u8; 8];
    #[cfg(unix)]
    {
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            let _ = f.read_exact(&mut buf);
        }
    }
    #[cfg(not(unix))]
    {
        use std::time::SystemTime;
        if let Ok(d) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            let nanos = d.as_nanos();
            buf = (nanos as u64).to_le_bytes();
        }
    }
    (u64::from_le_bytes(buf) as usize) % max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_count() {
        assert_eq!(THEMES.len(), 10);
    }

    #[test]
    fn all_themes_end_with_blank() {
        for (i, theme) in THEMES.iter().enumerate() {
            let last = theme.frames.last().expect("theme has no frames");
            assert!(
                last.trim().is_empty(),
                "theme {i} last frame is {last:?}, expected blank"
            );
        }
    }

    #[test]
    fn all_themes_have_minimum_frames() {
        for (i, theme) in THEMES.iter().enumerate() {
            assert!(
                theme.frames.len() >= 3,
                "theme {i} has only {} frames",
                theme.frames.len()
            );
        }
    }

    #[test]
    fn all_themes_have_valid_interval() {
        for (i, theme) in THEMES.iter().enumerate() {
            assert!(
                (80..=300).contains(&theme.interval_ms),
                "theme {i} interval {}ms outside 80-300ms range",
                theme.interval_ms
            );
        }
    }

    #[test]
    fn random_index_within_bounds() {
        for max in [1, 2, 5, 10, 100, 1000] {
            for _ in 0..50 {
                let idx = random_index(max);
                assert!(idx < max, "random_index({max}) returned {idx}");
            }
        }
    }

    #[test]
    fn random_index_with_one() {
        for _ in 0..20 {
            assert_eq!(random_index(1), 0);
        }
    }

    #[test]
    fn spinner_start_finish() {
        let sp = Spinner::start("test message");
        sp.finish();
    }
}
