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
                .template("  {spinner} {msg}")
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
