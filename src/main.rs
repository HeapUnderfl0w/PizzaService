extern crate chrono;
extern crate ears;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate toml;
extern crate fern;

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

mod model;

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    thread::{sleep as thread_sleep, spawn as thread_spawn},
};

use chrono::{Duration, Local};
use ears::{AudioController, Sound};
use std::sync::{Arc, Mutex};
use model::{Conf, Pizza};

fn main() {

    panic_with_message(setup_logging(), "We were unable to init logging (a rare thing)!");

    // Path to config file
    let confp: &Path = Path::new("x.toml");
    // Path to success audio file (atm defaulting to .ogg)
    let audio_file: &Path = Path::new("sound/success.ogg");

    // Check if the audio file is actually there and try to init ears
    let audio_mode = match audio_file.exists() {
        true => {
            info!("Found Audio-file !");
            match ears::init() {
                Ok(_) => {
                    info!("Initialized Audio !");
                    true
                }
                Err(e) => {
                    error!("Failed to initialize Audio");
                    error!("E: {:?}", e);
                    false
                }
            }
        }
        false => {
            info!("Unable to find audio file or init audio !");
            false
        }
    };

    // Can we find the config file ?
    if !confp.exists() {
        // We did not find one, so we create it
        let mut f = File::create(confp).unwrap();
        write!(f, "{}", toml::to_string(&Conf::default()).unwrap()).unwrap();
        // Panic to exit the program with a definitive error message
        panic!("No config");
    }

    // Load the config file
    let mut conf: Conf = {
        let mut f = File::open("x.toml").unwrap();
        let mut dta = String::new();
        f.read_to_string(&mut dta).unwrap();
        // TOML parsing made easy with Serde™
        toml::from_str(&dta).unwrap()
    };

    // Set varibales for the loop
    // Last printing of "I AM ALIVE !"
    let mut last_alive = Local::now();

    // Last change of config file
    let mut last_change = fs::metadata("x.toml").unwrap().modified().unwrap();

    // Already printed ID's (to avoid multiple prints of the same id)
    let mut pushed: Vec<u32> = Vec::new();

    // Lock that prohibits multiple sounds at the same time.
    let audio_lock: Arc<Mutex<()>> = Arc::new(Mutex::new(()));

    info!("Entering Main-Loop, Press Ctrl+C to cancle");

    loop {
        // Request new data from API
        match request_update(&conf.pizza.url) {
            Ok(o) => {
                // Iterate the pizzas
                for pizza in o {
                    if !pushed.contains(&pizza.id) && conf.pizza.to_watch.contains(&pizza.id) {
                        // We found a pizza that is watched & not in the list
                        info!("PIZZA {} [{}] READY !", pizza.name, pizza.id);
                        pushed.push(pizza.id);

                        // Check if we have audio mode
                        if audio_mode {
                            // Spawn a new thread that playes the success sound.
                            // the spawned thread will abort imediatly if a sound is
                            // already playing.
                            let c_audio_lock = audio_lock.clone();

                            thread_spawn(move || {
                                // Store the lock. when the thread exits the lock is dropped
                                // which will allow other threads to reaquire it.
                                // We are also using try lock as that means if we error
                                // another sound is already playing and this one dies not
                                // have to be queued.
                                let _audio_lock_guard = match c_audio_lock.try_lock() {
                                    Ok(guard) => guard,
                                    // Error means we are unable to lock
                                    // (a sound is probably already plaing)
                                    Err(_) => return,
                                };

                                let path = audio_file.clone();

                                // we were unable to find the audio file
                                // PANIC, ABORT, ABANDON SHIP !
                                if !path.exists() {
                                    error!(
                                        "Cannot find the audio file, did you move it ?"
                                    );
                                    return;
                                }

                                // Create new sound output
                                let mut snd = Sound::new(path.to_str().unwrap()).unwrap();

                                // PLAY THA SICK MUSIC !
                                snd.play();

                                // Avoid premature exit of thread while sound is playing
                                while snd.is_playing() {
                                    thread_sleep(Duration::milliseconds(100).to_std().unwrap());
                                }
                            });
                        }
                    }
                }
            }
            Err(e) => {
                // This is dissapointing, but can have numerous reasons (timeout, etc)
                // Just print the message and be done with it.
                error!("Invalid Response: {}", e);
            }
        }

        // Checked / Displayed all pizzas
        // Now check if we should print the next Alive Message

        // Probs make the duration here a config option but im lazy....
        if Local::now().signed_duration_since(last_alive) > Duration::minutes(1) {
            last_alive = Local::now();
            trace!(
                "I AM ALIIIIIIVE ! [{}]",
                last_alive.to_rfc2822() // Now with fancy Timestamp support
            );
        }

        // Check if changes to the config are longer than the threshhold
        let modt = fs::metadata("x.toml").unwrap().modified().unwrap();
        if modt.duration_since(last_change).unwrap()
            > Duration::milliseconds(conf.conf.refresh_conf as i64)
                .to_std()
                .unwrap()
        {
            // Threshhold reached, reload config !
            conf = {
                let mut f = File::open("x.toml").unwrap();
                let mut dta = String::new();
                f.read_to_string(&mut dta).unwrap();
                toml::from_str(&dta).unwrap()
            };
            last_change = modt;
            // Notify the user...
            warn!("Reloaded config !");
        }

        // pause for x milliseconds
        thread_sleep(
            Duration::milliseconds(conf.pizza.refresh as i64)
                .to_std()
                .unwrap(),
        );
    }
}

/// Request a new pizza update list from the api
fn request_update(url: &str) -> Result<Vec<Pizza>, String> {
    match reqwest::get(url) {
        Ok(mut o) => {
            // We got a response, try to load it !
            match o.json::<Vec<Pizza>>() {
                Ok(x) => Ok(x),
                Err(e) => Err(format!("ERR: {:?}", e)),
            }
        }
        // Eh, its probably a timeout again ¯\_(ツ)_/¯
        Err(e) => Err(format!("ERR: {:?}", e)),
    }
}

// -------------------------------------------
// Logging backend (log / fern)
fn setup_logging() -> Result<(), fern::InitError> {
    use fern::colors::{Color, ColoredLevelConfig};
    let log_colors = ColoredLevelConfig::new()
        .trace(Color::Blue)
        .debug(Color::BrightBlue)
        .info(Color::BrightWhite)
        .warn(Color::Yellow)
        .error(Color::BrightRed);
    ::fern::Dispatch::new()
        .format(move |o, m, r| {
            o.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
                r.target(),
                log_colors.color(r.level()),
                m
            ))
        })
        .level(log::LevelFilter::Info)
        .level_for(env!("CARGO_PKG_NAME"), log::LevelFilter::Trace)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

fn panic_with_message<T,E>(e: Result<T, E>, m: &str) where T: ::std::fmt::Debug, E: ::std::fmt::Debug {
    if let Err(ee) = e {
        println!("[FATAL ERROR] {}", m);
        panic!(format!("E {:?}", ee));
    }
}