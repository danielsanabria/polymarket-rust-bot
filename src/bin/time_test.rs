use chrono::prelude::*;
use chrono_tz::America::New_York;

fn main() {
    let now_utc = Utc::now();
    let now_et = now_utc.with_timezone(&New_York);
    println!("UTC: {}", now_utc);
    println!("ET: {}", now_et);
    println!("Year: {}, Month: {}, Day: {}, Hour: {}, Minute: {}", 
        now_et.year(), now_et.month(), now_et.day(), now_et.hour(), now_et.minute());
    
    let minute_floor = (now_et.minute() / 15) * 15;
    let period_start_et = New_York
        .with_ymd_and_hms(
            now_et.year(),
            now_et.month(),
            now_et.day(),
            now_et.hour(),
            minute_floor,
            0,
        )
        .single()
        .unwrap();
    println!("Period Start ET Timestamp: {}", period_start_et.timestamp());
}
