use chrono::Local;
use humantime::format_duration;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use log::{info, LevelFilter};
use simple_logging;
use std::env;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const LOGFILE_NAME: &str = "pomodoros.log";

struct App {
    test_mode: bool,
}

impl App {
    fn run(&mut self, interrupted: &AtomicBool) {
        self.read_args();
        self.run_timer(interrupted);
    }

    fn play_sound(duration: Duration) {
        use rodio::Sink;

        let device = rodio::default_output_device().unwrap();
        let sink = Sink::new(&device);

        sink.append(rodio::source::SineWave::new(440));
        std::thread::sleep(duration);
    }

    fn run_timer(&self, interrupted: &AtomicBool) {
        fn _info_and_print(msg: &String) {
            info!("{}", msg);
            println!("{}", msg);
        }

        let timer_duration: Duration;
        let mut was_interrupted: bool = false;

        if self.test_mode {
            timer_duration = Duration::from_secs(6);
        } else {
            timer_duration = chrono::Duration::minutes(25).to_std().unwrap();
        }

        let bar = ProgressBar::new(timer_duration.as_secs());
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} {spinner} [{eta_precise}] [{wide_bar:.red/red}]")
                .progress_chars("██ ")
                .tick_chars("🔴⚪ "),
        );
        bar.set_message("🍅");

        let one_second = Duration::from_secs(1);
        let start = Instant::now();
        info!(
            "🍅 Starting {} Pomodoro on {}",
            format_duration(timer_duration),
            Local::now().format("%A, %v at %H:%M:%S")
        );
        while (start.elapsed() < timer_duration) && !was_interrupted {
            std::thread::sleep(one_second);
            bar.inc(1);
            if interrupted.load(Ordering::SeqCst) {
                was_interrupted = true;
            }
        }

        bar.finish_and_clear();

        if was_interrupted {
            _info_and_print(&format!(
                "Interrupted at {} with {} remaining.",
                Local::now().format("%H:%M:%S"),
                HumanDuration(timer_duration - start.elapsed())
            ));
        } else {
            _info_and_print(&format!("Finished at {}", Local::now().format("%H:%M:%S")));
        }

        io::stdout().flush().unwrap();

        if !was_interrupted {
            // two beeps with a pause, to let my speaker wake up ...
            App::play_sound(one_second);
            std::thread::sleep(one_second);
            App::play_sound(one_second);
        }
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

    let irq = Arc::new(AtomicBool::new(false));

    let irq_c = irq.clone();
    ctrlc::set_handler(move || {
        irq_c.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let mut app = App { test_mode: false };

    app.run(&irq);
}
