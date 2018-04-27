extern crate chrono;
extern crate ears;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate toml;

#[macro_use]
extern crate serde_derive;

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    thread::{sleep as thread_sleep, spawn as thread_spawn},
};

use chrono::{Duration, Local};
use ears::{AudioController, Sound};
use std::sync::{Arc, Mutex};

fn main() {
    // Path to config file
    let confp: &Path = Path::new("x.toml");
    // Path to success audio file (atm defaulting to .ogg)
    let audio_file: &Path = Path::new("success.ogg");

    // Check if the audio file is actually there and try to init ears
    let audio_mode = match audio_file.exists() && ears::init() {
        true => {
            println!("[INFO ] Found the audio file and initialized the audio !");
            true
        }
        false => {
            println!("[INFO ] Unable to find audio file or init audio !");
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
    loop {
        // Request new data from API
        match request_update(&conf.pizza.url) {
            Ok(o) => {
                // Iterate the pizzas
                for pizza in o {
                    if !pushed.contains(&pizza.id) && conf.pizza.to_watch.contains(&pizza.id) {
                        // We found a pizza that is watched & not in the list
                        println!("[INFO ] PIZZA {} [{}] READY !", pizza.name, pizza.id);
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
                                    println!(
                                        "[ERROR ] Cannot find the audio file, did you move it ?"
                                    );
                                    return;
                                }

                                // Create new sound output
                                let mut snd = Sound::new(path.to_str().unwrap()).unwrap();

                                // PLAY THA SICK MUSIC !
                                snd.play();

                                // Avoid premature exit of thread while sound is playing
                                while snd.is_playing() {}
                            });
                        }
                    }
                }
            }
            Err(e) => {
                // This is dissapointing, but can have numerous reasons (timeout, etc)
                // Just print the message and be done with it.
                println!("[ERROR] Invalid Response: {}", e);
            }
        }

        // Checked / Displayed all pizzas
        // Now check if we should print the next Alive Message

        // Probs make the duration here a config option but im lazy....
        if Local::now().signed_duration_since(last_alive) > Duration::minutes(1) {
            last_alive = Local::now();
            println!(
                "[ECHO ] I AM ALIIIIIIVE ! [{}]",
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
            println!("[WARN ] Reloaded config !");
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

// Pizza object (represents id & type of the pizza)
#[derive(Debug, Serialize, Deserialize)]
struct Pizza {
    pub id: u32,
    #[serde(rename = "pizza")]
    pub name: String,
}

// Config root (cause i want it to look fancy)
#[derive(Debug, Serialize, Deserialize)]
struct Conf {
    conf:  ConfConf,
    pizza: PizzaConf,
}

// Configuration config
#[derive(Debug, Serialize, Deserialize)]
struct ConfConf {
    refresh_conf: u32,
}

// DEFAULTS, FUCK YEA
impl Default for Conf {
    fn default() -> Self {
        Conf {
            conf:  ConfConf::default(),
            pizza: PizzaConf::default(),
        }
    }
}

// DEFAULTS, FUCK YEA
impl Default for ConfConf {
    fn default() -> Self { ConfConf { refresh_conf: 1000 } }
}

// Pizza api conf
// (do not set refresh lower than 200, could be kinda spammy xD)
#[derive(Debug, Serialize, Deserialize)]
struct PizzaConf {
    url:      String,
    refresh:  u32,
    to_watch: Vec<u32>,
}

// DEFAULTS, FUCK YEA
impl Default for PizzaConf {
    fn default() -> Self {
        Self {
            url:      String::from(""),
            refresh:  500,
            to_watch: Vec::new(),
        }
    }
}
