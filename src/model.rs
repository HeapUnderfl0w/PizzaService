// Pizza object (represents id & type of the pizza)
#[derive(Debug, Serialize, Deserialize)]
pub struct Pizza {
    pub id: u32,
    #[serde(rename = "pizza")]
    pub name: String,
}

// Config root (cause i want it to look fancy)
#[derive(Debug, Serialize, Deserialize)]
pub struct Conf {
    pub conf:  ConfConf,
    pub pizza: PizzaConf,
}

// Configuration config
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfConf {
    pub refresh_conf: u32,
}

// Pizza api conf
// (do not set refresh lower than 200, could be kinda spammy xD)
#[derive(Debug, Serialize, Deserialize)]
pub struct PizzaConf {
    pub url:      String,
    pub refresh:  u32,
    pub to_watch: Vec<u32>,
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
