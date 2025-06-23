use log::LevelFilter;
use rummy::logger::init_logger;

fn main() {
    init_logger(LevelFilter::Warn, "default.log").expect("Logger init failed");
    println!("Hello, world!");
}