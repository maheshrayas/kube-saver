use chrono_tz::Tz;
use k8s_openapi::chrono::{
    Date, DateTime, Datelike, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc,
};
use log::{debug, error, info};
use regex::Captures;
use std::{process::exit, str::FromStr};

use crate::util::Error;

pub fn current_day(day: &str) -> u32 {
    match day {
        "Mon" => 0,
        "Tue" => 1,
        "Wed" => 2,
        "Thu" => 3,
        "Fri" => 4,
        "Sat" => 5,
        "Sun" => 6,
        _ => exit(1),
    }
}

struct Timer {
    dt: DateTime<Tz>,
    config_date_low_hour: NaiveDateTime,
    config_date_high_hour: NaiveDateTime,
}
impl Timer {
    pub fn cmp_time(&self) -> bool {
        if self.dt.naive_local() > self.config_date_low_hour
            && self.dt.naive_local() < self.config_date_high_hour
        {
            // the uptime is between the range
            // start upscaling
            info!(
                "current {} is range  {} &  {}",
                self.dt, self.config_date_low_hour, self.config_date_high_hour
            );
            true
        } else {
            // the downtime is between the range
            // start downscaling
            info!(
                "current {} is not in range  {} &  {}",
                self.dt, self.config_date_low_hour, self.config_date_high_hour
            );
            debug!("Current rules states, its a downtime for configured resources");
            false
        }
    }
}

pub fn is_uptime(m: Captures) -> Result<bool, Error> {
    let week_start = current_day(&m[1]); //MON 0
    let week_end = current_day(&m[2]); // FRI 4
    let low_hour: u32 = FromStr::from_str(&m[3]).unwrap(); // 07
    let low_min: u32 = FromStr::from_str(&m[4]).unwrap(); // 10
    let high_hour: u32 = FromStr::from_str(&m[5]).unwrap(); // 01
    let high_min: u32 = FromStr::from_str(&m[6]).unwrap(); // 00
    let config_tz: &str = &m["tz"]; // AEST
    let tz: Tz = config_tz.parse()?;
    // get the current datetime based on the timezone
    let dt: DateTime<Tz> = Utc::now().with_timezone(&tz); // 00
                                                          // get the current time
                                                          // this is hack to check if end time > 12:00 AM and complx hack for downscaling.
    let complex_high_time = Utc.ymd(2020, 1, 1).and_hms(high_hour, high_min, 0);
    let complex_low_time = Utc.ymd(2020, 1, 1).and_hms(low_hour, low_min, 0);

    // current datetime which is updated as per conditions

    let mut config_date_low_hour =
        NaiveDate::from_ymd(dt.year(), dt.month(), dt.day()).and_hms_milli(low_hour, low_min, 0, 0);
    let mut config_date_high_hour = NaiveDate::from_ymd(dt.year(), dt.month(), dt.day())
        .and_hms_milli(high_hour, high_min, 0, 0);

    // check if the current day is configured in the input week range
    if dt.weekday().num_days_from_monday() >= week_start
        && dt.weekday().num_days_from_monday() <= week_end
        && complex_high_time > complex_low_time
    {
        // if the current date time is greater or equal to current date low hour and current date time is less than or equal to current date high hour.
        println!("config_date_low_hour {}", config_date_low_hour);
        println!("config_date_high_hour {}", config_date_high_hour);
        println!("current utc {}", dt.naive_local());
        let t = Timer {
            dt,
            config_date_low_hour,
            config_date_high_hour,
        };
        Ok(t.cmp_time())
    } else if complex_high_time < complex_low_time {
        println!("start hack");
        // check if current day has passed the end day of rule
        // for example RULE = Mon-Fri 7AM - 01AM, and its sat 01:10 AM
        if dt.weekday().num_days_from_monday() == week_end + 1 {
            config_date_low_hour = NaiveDate::from_ymd(dt.year(), dt.month(), dt.day() - 1)
                .and_hms_milli(low_hour, low_min, 0, 0);
            let t = Timer {
                dt,
                config_date_low_hour,
                config_date_high_hour,
            };
            return Ok(t.cmp_time());
        }
        // for example RULE = Mon-Fri 7 - 02 AM, and its Mon 1 AM
        else if dt.weekday().num_days_from_monday() == week_start
            && dt.hour() < complex_high_time.hour()
        {
            // downscaling
            info!("Current rules states, its a downtime for configured resources");
            return Ok(false);
        } else if dt.weekday().num_days_from_monday() >= week_start
            && dt.weekday().num_days_from_monday() <= week_end
        {
            // if current time has crossed 12 AM but less than or equals to high hour
            if dt.hour() <= complex_high_time.hour() {
                //below condition is needed if minutes are involved, for example scale down is 2:30 AM
                config_date_low_hour = NaiveDate::from_ymd(dt.year(), dt.month(), dt.day() - 1)
                    .and_hms_milli(low_hour, low_min, 0, 0);
                let t = Timer {
                    dt,
                    config_date_low_hour,
                    config_date_high_hour,
                };
                info!(" dt.hour() <= complex_high_time.hour()");
                return Ok(t.cmp_time());
            }
            // if current time is less 12 AM but less than low hour
            else if dt.hour() >= complex_low_time.hour() {
                // below is needed if there are minutes involved, for example start time is 7:30 AM
                config_date_high_hour = NaiveDate::from_ymd(dt.year(), dt.month(), dt.day() + 1)
                    .and_hms_milli(high_hour, high_min, 0, 0);
                let t = Timer {
                    dt,
                    config_date_low_hour,
                    config_date_high_hour,
                };
                info!("dt.hour() >= complex_low_time.hour()");
                return Ok(t.cmp_time());
            }
        }
        return Ok(false);
    } else {
        println!("not comming here");
        // current day is not configured in the uptime
        debug!("current day is not configured in the uptime,hence downscaling");
        Ok(false)
    }
}
