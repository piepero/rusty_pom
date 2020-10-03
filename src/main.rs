use chrono::Local;
use humantime::format_duration;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, LevelFilter};
use simple_logging;
use std::env;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::time::{Duration, Instant};

const LOGFILE_NAME: &str = "pomodoros.log";

struct App {
    test_mode: bool,
}

impl App {
    fn run(&mut self) {
        self.read_args();
        self.run_pomodoro();
    }

    fn play_sound(&self, duration: Duration) {
        use rodio::Sink;

        let device = rodio::default_output_device().unwrap();
        let sink = Sink::new(&device);

        // Add a dummy source of the sake of the example.
        let source = rodio::source::SineWave::new(440);
        std::thread::sleep(Duration::from_secs(1));
        sink.append(source);
        std::thread::sleep(duration);
    }

    fn run_pomodoro(&self) {
        let mini_pomodoro: Duration;

        if self.test_mode {
            mini_pomodoro = Duration::from_secs(6);
        } else {
            mini_pomodoro = chrono::Duration::minutes(25).to_std().unwrap();
        }
        let bar = ProgressBar::new(mini_pomodoro.as_secs());
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} {spinner} [{eta_precise}] [{wide_bar:.red/red}]")
                .progress_chars("██ ")
                .tick_chars("🔴⚪ "),
        );
        bar.set_message("🍅");

        let tick = Duration::from_secs(1);
        let start = Instant::now();
        info!(
            "🍅 Starting {} Pomodoro on {}",
            format_duration(mini_pomodoro),
            Local::now().format("%A, %v at %H:%M:%S")
        );
        while start.elapsed() < mini_pomodoro {
            std::thread::sleep(tick);
            bar.inc(1);
        }
        info!("Finished at {}", Local::now().format("%H:%M:%S"));
        bar.finish_and_clear();
        print!("Finished at {}", Local::now().format("%H:%M:%S"));
        io::stdout().flush().unwrap();

        // zwei Töne, damit meine Soundkarte aufwachen kann ...
        self.play_sound(Duration::from_secs(2));
        std::thread::sleep(tick);
        self.play_sound(Duration::from_secs(1));
    }

    fn read_args(&mut self) {
        let args: Vec<String> = env::args().collect();
        // println!("Args: {:?}", args);

        self.test_mode = args.contains(&"--test".to_string());
    }
}

/// Configure logging, initialize the app, and run it.
fn main() {
    simple_logging::log_to(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(LOGFILE_NAME)
            .unwrap(),
        LevelFilter::Info,
    );

    let mut app = App { test_mode: false };

    app.run();
}
