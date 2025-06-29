use std::env::args;

use epithet_2::epithet_config::EpithetConfig;

fn main() {
    let config = EpithetConfig::read(&args().nth(1).unwrap()).unwrap();
    let args = args().skip(2).collect::<Vec<String>>();

    dbg!(&config);
    println!("--------------------------------");

    config.execute("y", &args).unwrap();
}
