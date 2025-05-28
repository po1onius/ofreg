pub mod time {
    use chrono::{DateTime, Utc};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn get_system_boot_time() -> i64 {
        let uptime_seconds = std::fs::read_to_string("/proc/uptime")
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<f64>()
            .unwrap() as i64;
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - uptime_seconds
    }

    fn jiffies_to_seconds(jiffies: u64, hz: u64) -> i64 {
        (jiffies / hz) as i64
    }

    fn calculate_process_time(boot_time: i64, start_time_jiffies: u64) -> i64 {
        boot_time + jiffies_to_seconds(start_time_jiffies, 100)
    }

    fn timestamp_to_ymd(timestamp: i64) -> String {
        let naive = DateTime::from_timestamp(timestamp, 0).unwrap();
        return naive.to_string();
    }

    pub fn jeff2time(start_time_jiffies: u64) -> String {
        let boot_time = get_system_boot_time();
        let process_timestamp = calculate_process_time(boot_time, start_time_jiffies);

        timestamp_to_ymd(process_timestamp)
    }
}
