use log::LevelFilter;
use rand::random;
use rummy::logger::init_logger;

#[test]
fn log_test() {
    init_logger(LevelFilter::Info, "myapp.log").expect("Logger init failed");

    let start = std::time::Instant::now();

    for i in 0..10_000 {
        let random_num = random::<u32>();
        match random_num % 4 {
            0 => log::debug!("Debug message {}", i),
            1 => log::info!("Info message {}", i),
            2 => log::warn!("Warning message {}", i),
            _ => log::error!("Error message {}", i),
        }
    }

    let duration = start.elapsed();
    println!("写入 1 万条日志用时：{:?}", duration);
    assert!(
        duration.as_millis() < 150,
        "日志写入耗时超过150ms，实际耗时: {:?}",
        duration
    );
}