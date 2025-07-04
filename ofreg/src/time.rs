use anyhow::anyhow;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};

pub fn human_time_to_sec(time_str: &str) -> anyhow::Result<u64> {
    let naive_time = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S")?;

    let local_time = Local.from_local_datetime(&naive_time).unwrap();
    let utc_time = local_time.with_timezone(&Utc);

    // let unix_epoch = NaiveDate::from_ymd_opt(1970, 1, 1)
    //     .ok_or(anyhow!(""))?
    //     .and_hms_opt(0, 0, 0)
    //     .ok_or(anyhow!(""))?;

    // let seconds = datetime.signed_duration_since(unix_epoch).num_seconds();
    let seconds = utc_time.timestamp();
    Ok(seconds as u64)
}

pub fn sec_to_human_time(timestamp: u64) -> anyhow::Result<String> {
    let datetime_utc = DateTime::from_timestamp(timestamp as i64, 0).ok_or(anyhow!(""))?;

    let datetime_local = datetime_utc.with_timezone(&chrono::Local);
    Ok(datetime_local.format("%Y-%m-%d %H:%M:%S %Z").to_string())
}
