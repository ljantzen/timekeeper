use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};

pub fn local_midnight_utc(date: NaiveDate) -> DateTime<Utc> {
    Local
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .unwrap()
        .with_timezone(&Utc)
}
