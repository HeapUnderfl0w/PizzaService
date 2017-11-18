extern crate serde;
extern crate toml;
extern crate reqwest;
extern crate serde_json;
extern crate chrono;
extern crate ears;

#[macro_use]
extern crate serde_derive;


use std::fs::File;
use std::fs;
use std::path::Path;
use std::io::Write;
use std::io::Read;
use std::thread::sleep as thread_sleep;
use std::thread::spawn as thread_spawn;
use chrono::Duration;
use chrono::Local;
use ears::Sound;
use ears::AudioController;


fn main() {
    let confp: &Path = Path::new("x.toml");
    let audio_file: &Path = Path::new("success.ogx");

    let AUDIO_MODE = audio_file.exists() && ears::init();

    if !confp.exists() {
        let mut f = File::create(confp).unwrap();
        write!(f, "{}", toml::to_string(&Conf::default()).unwrap())
            .unwrap();
        panic!("No config");
    }

    let mut conf: Conf = {
        let mut f = File::open("x.toml").unwrap();
        let mut dta = String::new();
        f.read_to_string(&mut dta).unwrap();
        toml::from_str(&dta).unwrap()
    };
    let mut last_alive = Local::now();
    let mut last_change = fs::metadata("x.toml").unwrap().modified().unwrap();

    let mut pushed: Vec<u32> = Vec::new();
    loop {
        match request_update(&conf.pizza.url) {
            Ok(o) => {
                for pizza in o {
                    if !pushed.contains(&pizza.id) && conf.pizza.to_watch.contains(&pizza.id) {
                        println!("[INFO ] PIZZA {} [{}] READY !", pizza.name, pizza.id);
                        pushed.push(pizza.id);
                        if AUDIO_MODE {
                            thread_spawn(move || {
                                let path = audio_file.clone();
                                let mut snd = Sound::new(path.to_str().unwrap()).unwrap();
                                snd.play();

                                while snd.is_playing() {}
                            });
                        }
                    }
                }
            },
            Err(e) => {
                println!("[ERROR] Invalid Response: {}", e);
            },
        }
        if Local::now().signed_duration_since(last_alive) > Duration::minutes(1) {
            last_alive = Local::now();
            println!(
                "[ECHO ] I AM STILL ALIIIIIIVE ! [{}]",
                last_alive.to_rfc2822()
            );
        }
        let modt = fs::metadata("x.toml").unwrap().modified().unwrap();
        if modt.duration_since(last_change).unwrap() >
           Duration::milliseconds(conf.conf.refresh_conf as i64)
               .to_std()
               .unwrap() {
            conf = {
                let mut f = File::open("x.toml").unwrap();
                let mut dta = String::new();
                f.read_to_string(&mut dta).unwrap();
                toml::from_str(&dta).unwrap()
            };
            last_change = modt;
            println!("[WARN ] Reloaded config !");
        }

        thread_sleep(
            Duration::milliseconds(conf.pizza.refresh as i64)
                .to_std()
                .unwrap(),
        );
    }
}

fn request_update(url: &str) -> Result<Vec<Pizza>, String> {
    match reqwest::get(url) {
        Ok(mut o) => {
            match o.json::<Vec<Pizza>>() {
                Ok(x) => Ok(x),
                Err(e) => Err(format!("ERR: {:?}", e)),
            }
        },
        Err(e) => Err(format!("ERR: {:?}", e)),
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct Pizza {
    pub id: u32,
    #[serde(rename = "pizza")]
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Conf {
    conf: ConfConf,
    pizza: PizzaConf,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfConf {
    refresh_conf: u32,
}

impl Default for Conf {
    fn default() -> Self {
        Conf {
            conf: ConfConf::default(),
            pizza: PizzaConf::default(),
        }
    }
}

impl Default for ConfConf {
    fn default() -> Self {
        ConfConf {
            refresh_conf: 1000,
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct PizzaConf {
    url: String,
    refresh: u32,
    to_watch: Vec<u32>,
}

impl Default for PizzaConf {
    fn default() -> Self {
        Self {
            url: String::from(""),
            refresh: 500,
            to_watch: Vec::new(),
        }
    }
}
