use chrono::Local;
use log::Level;
use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::sync::{mpsc, OnceLock};
use std::thread;

struct DefaultLogger {
    level_filter: LevelFilter,
    file_sender: mpsc::Sender<String>,
    console_sender: mpsc::Sender<(Level, String)>,
}

impl Log for DefaultLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level_filter.to_level().unwrap_or(Level::Error)
    }

    fn log(&self, record: &Record) {
        let now = Local::now();
        let msg = format!(
            "[{}][{}] {}\n",
            now.format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.args()
        );

        // 发送日志给写文件线程（全部写）
        let _ = self.file_sender.send(msg.clone());

        // 控制台打印策略：
        // Error 级别立即打印，其它等级缓存10条批量打印
        let _ = self.console_sender.send((record.level(), msg));
    }

    fn flush(&self) {
        // 如果需要实现 flush，可以发送特殊消息让线程强制刷新
    }
}

static LOGGER: OnceLock<DefaultLogger> = OnceLock::new();

pub fn init_logger(level: LevelFilter, file_path: &str) -> Result<(), SetLoggerError> {
    let (file_tx, file_rx) = mpsc::channel::<String>();
    let (console_tx, console_rx) = mpsc::channel::<(Level, String)>();

    // 打开日志文件
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .expect("Failed to open log file");

    // 后台写文件线程（同步简单版）
    thread::spawn(move || {
        let mut file = file;
        for line in file_rx {
            if let Err(e) = file.write_all(line.as_bytes()) {
                eprintln!("Log write error: {}", e);
            }
        }
        let _ = file.flush();
    });

    // 后台控制台打印线程，批量处理
    thread::spawn(move || {
        let mut buffer:Vec<String> = Vec::with_capacity(10);
        loop {
            // 阻塞接收
            match console_rx.recv() {
                Ok((level, msg)) => {
                    if level == Level::Error {
                        // Error 立即打印
                        let _ = io::stderr().write_all(msg.as_bytes());
                        // 如果缓冲区里有积累的，先批量打印
                        if !buffer.is_empty() {
                            for m in buffer.drain(..) {
                                let _ = io::stderr().write_all(m.as_bytes());
                            }
                        }
                    } else {
                        // 普通等级缓冲
                        buffer.push(msg);
                        if buffer.len() >= 10 {
                            for m in buffer.drain(..) {
                                let _ = io::stderr().write_all(m.as_bytes());
                            }
                        }
                    }
                }
                Err(_) => {
                    // 发送端关闭，打印剩余
                    for m in buffer.drain(..) {
                        let _ = io::stderr().write_all(m.as_bytes());
                    }
                    break;
                }
            }
        }
    });

    let logger = DefaultLogger {
        level_filter: level,
        file_sender: file_tx,
        console_sender: console_tx,
    };

    let _ = LOGGER.set(logger);
    log::set_logger(LOGGER.get().unwrap())?;
    log::set_max_level(level);
    Ok(())
}