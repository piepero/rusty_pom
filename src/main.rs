use chrono::Local;
use humantime::format_duration;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use log::{info, LevelFilter};
use serde::{Deserialize, Serialize};
use serde_json;
use simple_logging;
use std::env;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const LOGFILE_NAME: &str = "pomodoros.log";
const STATEFILE_NAME: &str = ".rusty_pom";

#[derive(Serialize, Deserialize)]
struct SavedState {
    seconds_remaining: u64,
}

struct App<'a> {
    arg_test_mode: bool,
    arg_restart: bool,
    ctrl_pressed: &'a AtomicBool,
    saved_state: &'a SavedState,
}

impl App<'_> {
    fn run(&mut self) {
        self.read_args();
        self.run_timer();
    }

    fn play_sound(duration: Duration) {
        use rodio::Sink;

        let device = rodio::default_output_device().unwrap();
        let sink = Sink::new(&device);

        sink.append(rodio::source::SineWave::new(440));
        std::thread::sleep(duration);
    }

    fn save_state(secs_remaining: u64) {
        let mut output = File::create(STATEFILE_NAME).expect("cannot create state file");
        let state = SavedState {
            seconds_remaining: secs_remaining,
        };
        write!(output, "{}", &serde_json::to_string(&state).unwrap())
            .expect("error writing to state file");
    }

    fn run_timer(&self) {
        fn _info_and_print(msg: &String) {
            info!("{}", msg);
            println!("{}", msg);
        }

        let timer_duration: Duration;
        let mut was_interrupted: bool = false;
        let mut was_continued: bool = false;
        let mut symbol = "🍅";

        if self.arg_test_mode {
            // only 6 second "pomodoros" in test mode; long enough to interrupt, short enough to let it finish
            timer_duration = Duration::from_secs(6);
        } else {
            if (self.saved_state.seconds_remaining > 0) && !self.arg_restart {
                timer_duration = Duration::from_secs(self.saved_state.seconds_remaining);
                was_continued = true;
                symbol = "🍏";
            } else {
                timer_duration = Duration::from_secs(25 * 60);
            }
        }

        let bar = ProgressBar::new(timer_duration.as_secs());
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} {spinner} [{eta_precise}] [{wide_bar:.red/red}]")
                .progress_chars("██ ")
                .tick_chars("🔴⚪ "),
        );
        bar.set_message(symbol);

        let one_second = Duration::from_secs(1);
        let start = Instant::now();

        info!(
            "{} {} {} Pomodoro on {}",
            symbol,
            if was_continued {
                "Continuing"
            } else {
                "Starting new"
            },
            format_duration(timer_duration),
            Local::now().format("%A, %v at %H:%M:%S")
        );

        while (start.elapsed() < timer_duration) && !was_interrupted {
            std::thread::sleep(one_second);
            bar.inc(1);
            if self.ctrl_pressed.load(Ordering::SeqCst) {
                was_interrupted = true;
            }
        }

        bar.finish_and_clear();

        if was_interrupted {
            let time_remaining = timer_duration - start.elapsed();

            _info_and_print(&format!(
                "Interrupted at {} with {} remaining.",
                Local::now().format("%H:%M:%S"),
                HumanDuration(time_remaining)
            ));
            App::save_state(time_remaining.as_secs());
        } else {
            _info_and_print(&format!("Finished at {}", Local::now().format("%H:%M:%S")));
            App::save_state(0);
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
        // TODO: use crate for proper argument handling
        let args: Vec<String> = env::args().collect();
        // println!("Args: {:?}", args);

        self.arg_test_mode = args.contains(&"--test".to_string());
        self.arg_restart = args.contains(&"--restart".to_string());
    }
}

/// Configure logging, initialize the app, and run it.
fn main() {
    fn get_saved_state(state: &mut SavedState) {
        let input = File::open(STATEFILE_NAME);

        let temp_state: SavedState = match input {
            Ok(input) => serde_json::from_reader(input).expect("error while reading json"),
            Err(_e) => SavedState {
                seconds_remaining: 0,
            },
        };
        state.seconds_remaining = temp_state.seconds_remaining;
    }

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

    let mut last_state = SavedState {
        seconds_remaining: 0,
    };
    get_saved_state(&mut last_state);

    let mut app = App {
        arg_test_mode: false,
        arg_restart: false,
        ctrl_pressed: &irq,
        saved_state: &last_state,
    };

    app.run();
}
