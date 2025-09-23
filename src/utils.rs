use std::sync::OnceLock;
use std::time::SystemTime;

// 타임존 오프셋 캐시
static TIMEZONE_OFFSET: OnceLock<i64> = OnceLock::new();

pub fn format_file_size(size: Option<u64>) -> String {
    match size {
        None => "     ".to_string(),
        Some(size) => {
            if size < 1024 {
                format!("{:>4}B", size)
            } else if size < 1024 * 1024 {
                format!("{:>4.1}K", size as f64 / 1024.0)
            } else if size < 1024 * 1024 * 1024 {
                format!("{:>4.1}M", size as f64 / (1024.0 * 1024.0))
            } else {
                format!("{:>4.1}G", size as f64 / (1024.0 * 1024.0 * 1024.0))
            }
        }
    }
}

pub fn format_modified_time(time: Option<SystemTime>) -> String {
    match time {
        None => "           ".to_string(),
        Some(time) => match time.duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => {
                let timestamp = duration.as_secs();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let local_offset = get_local_timezone_offset();
                let local_timestamp = if local_offset >= 0 {
                    timestamp + local_offset as u64
                } else {
                    timestamp.saturating_sub((-local_offset) as u64)
                };
                let local_now = if local_offset >= 0 {
                    now + local_offset as u64
                } else {
                    now.saturating_sub((-local_offset) as u64)
                };

                let days_since_epoch = local_timestamp / 86400;
                let _current_days = local_now / 86400;

                let mut year = 1970;
                let mut remaining_days = days_since_epoch;

                loop {
                    let days_in_year = if is_leap_year(year) { 366 } else { 365 };
                    if remaining_days >= days_in_year {
                        remaining_days -= days_in_year;
                        year += 1;
                    } else {
                        break;
                    }
                }

                let mut month = 1;
                let days_in_months = if is_leap_year(year) {
                    [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                } else {
                    [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                };

                for &days_in_month in &days_in_months {
                    if remaining_days >= days_in_month {
                        remaining_days -= days_in_month;
                        month += 1;
                    } else {
                        break;
                    }
                }
                let day = remaining_days + 1;

                let secs_today = local_timestamp % 86400;
                let hours = secs_today / 3600;
                let minutes = (secs_today % 3600) / 60;

                let month_names = [
                    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
                    "Dec",
                ];
                let month_name = month_names[(month - 1).min(11) as usize];

                let current_year = 1970 + (local_now / (365 * 86400));

                if year == current_year {
                    format!("{} {:2} {:02}:{:02}", month_name, day, hours, minutes)
                } else {
                    format!("{} {:2}  {:04}", month_name, day, year)
                }
            }
            Err(_) => "           ".to_string(),
        },
    }
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn get_local_timezone_offset() -> i64 {
    *TIMEZONE_OFFSET.get_or_init(|| {
        use std::process::Command;

        if let Ok(output) = Command::new("date").arg("+%z").output() {
            if let Ok(offset_str) = String::from_utf8(output.stdout) {
                let offset_str = offset_str.trim();
                if offset_str.len() >= 5 {
                    let sign = if offset_str.starts_with('+') { 1 } else { -1 };
                    if let (Ok(hours), Ok(minutes)) = (
                        offset_str[1..3].parse::<i32>(),
                        offset_str[3..5].parse::<i32>(),
                    ) {
                        let total_minutes = hours * 60 + minutes;
                        let offset_seconds = sign * total_minutes * 60;
                        return offset_seconds as i64;
                    }
                }
            }
        }

        0
    })
}

pub fn truncate_path(path: &str, max_width: usize) -> String {
    if path.len() <= max_width {
        return path.to_string();
    }

    if max_width < 3 {
        return "...".to_string();
    }

    let start_len = (max_width - 3) / 2;
    let end_len = max_width - 3 - start_len;

    format!("{}...{}", &path[..start_len], &path[path.len() - end_len..])
}

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::io::Write;

static LOGGING_ENABLED: AtomicBool = AtomicBool::new(false);
static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

pub fn init_logging(verbose: bool) {
    LOGGING_ENABLED.store(verbose, Ordering::Relaxed);

    if verbose {
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("tudiff.log")
        {
            *LOG_FILE.lock().unwrap() = Some(file);
            log_info("Logging initialized");
        }
    }
}

pub fn log_error(message: &str) {
    if LOGGING_ENABLED.load(Ordering::Relaxed) {
        log_with_level("ERROR", message);
    }
}

pub fn log_info(message: &str) {
    if LOGGING_ENABLED.load(Ordering::Relaxed) {
        log_with_level("INFO", message);
    }
}

pub fn log_debug(message: &str) {
    if LOGGING_ENABLED.load(Ordering::Relaxed) {
        log_with_level("DEBUG", message);
    }
}

fn log_with_level(level: &str, message: &str) {
    let log_message = format!(
        "[{}] {}: {}\n",
        level,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        message
    );

    if let Ok(mut log_file_guard) = LOG_FILE.lock() {
        if let Some(ref mut file) = *log_file_guard {
            let _ = file.write_all(log_message.as_bytes());
            let _ = file.flush();
        }
    }
}
