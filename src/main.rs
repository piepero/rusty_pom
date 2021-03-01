extern crate winrt_notification;

use chrono::Local;
use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};
use humantime::format_duration;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use log::{info, LevelFilter};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winrt_notification::{Duration as WinRtDuration, Sound, Toast};

const LOGFILE_NAME: &str = "pomodoros.log";
const STATEFILE_NAME: &str = ".rusty_pom";

#[derive(Serialize, Deserialize)]
struct SavedState {
    seconds_remaining: u64,
}

struct PomApp<'a> {
    arg_restart: bool,
    arg_duration: i32,
    ctrl_pressed: &'a AtomicBool,
    saved_state: &'a SavedState,
}

impl PomApp<'_> {
    fn run(&mut self) {
        self.run_timer();
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
        fn _info_and_print(msg: &str) {
            info!("{}", msg);
            println!("{}", msg);
        }

        let timer_duration: Duration;
        let mut was_interrupted: bool = false;
        let mut was_continued: bool = false;
        let mut symbol = "🍅";

        if (self.saved_state.seconds_remaining > 0) && !self.arg_restart {
            timer_duration = Duration::from_secs(self.saved_state.seconds_remaining);
            was_continued = true;
            symbol = "🍏";
        } else if self.arg_duration > 0 {
            timer_duration = Duration::from_secs(u64::try_from(self.arg_duration).unwrap() * 60)
        } else {
            timer_duration = Duration::from_secs(u64::try_from(-self.arg_duration).unwrap())
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
            PomApp::save_state(time_remaining.as_secs());
        } else {
            _info_and_print(&format!("Finished at {}", Local::now().format("%H:%M:%S")));
            PomApp::save_state(0);
        }

        io::stdout().flush().unwrap();

        if !was_interrupted {
            Toast::new(Toast::POWERSHELL_APP_ID)
                .title("Pomodoro finished!")
                .text1("Your pomodoro has finished.")
                .sound(Some(Sound::Reminder))
                .duration(WinRtDuration::Short)
                .show()
                .expect("unable to toast");
        }
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

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::new("duration")
                .short('d')
                .long("duration")
                .about("Duration in minutes, defaults to 25")
                .takes_value(true)
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new("restart")
                .short('r')
                .long("restart")
                .about("Restart a new pomodoro"),
        )
        .get_matches();

    let duration: i32 = matches
        .value_of("duration")
        .unwrap_or("25")
        .parse()
        .unwrap();
    println!("Value for duration: {}", duration);

    let mut app: PomApp = PomApp {
        arg_restart: matches.is_present("restart"),
        arg_duration: duration,
        ctrl_pressed: &irq,
        saved_state: &last_state,
    };

    app.run();
}
