use std::env::args;

use epithet_2::epithet_config::EpithetConfig;

fn main() {
    let config = EpithetConfig::read(&args().nth(1).unwrap()).unwrap();

    dbg!(&config);
    println!("--------------------------------");
    let fake = EpithetConfig::fake();
    println!("{}", toml::to_string_pretty(&fake).unwrap());
    println!("--------------------------------");

    config.execute("y").unwrap();
}
