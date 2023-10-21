use chrono::{DateTime, Datelike, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use log::{debug, info};
use regex::Captures;
use std::{process::exit, str::FromStr};
use tracing::error;

use crate::error::Error;

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
            && self.dt.naive_local() <= self.config_date_high_hour
        {
            // the uptime is between the range
            // start upscaling
            debug!(
                "current {} is range  {} &  {}",
                self.dt, self.config_date_low_hour, self.config_date_high_hour
            );
            true
        } else {
            // the downtime is between the range
            // start downscaling
            debug!(
                "current {} is not in range  {} &  {}",
                self.dt, self.config_date_low_hour, self.config_date_high_hour
            );
            debug!("Current rules states, its a downtime for configured resources");
            false
        }
    }
}

struct UpTimeCheck {
    week_start: u32,
    week_end: u32,
    low_hour: u32,
    low_min: u32,
    high_hour: u32,
    high_min: u32,
    dt: DateTime<Tz>,
}

impl UpTimeCheck {
    fn is_uptime(&self) -> Result<bool, Error> {
        // this is hack to check if end time > 12:00 AM and complx hack for downscaling.
        // For example start time Mon 7AM  - 2 AM
        let complex_high_time = if let LocalResult::None =
            Utc.with_ymd_and_hms(2020, 1, 1, self.high_hour, self.high_min, 0)
        {
            error!("Invalid time {}:{}", self.high_hour, self.high_min);
            return Err(Error::UserInputError("Invalid time".to_string()));
        } else {
            Utc.with_ymd_and_hms(2020, 1, 1, self.high_hour, self.high_min, 0)
                .unwrap()
        };

        let complex_low_time = if let LocalResult::None =
            Utc.with_ymd_and_hms(2020, 1, 1, self.low_hour, self.low_min, 0)
        {
            error!("Invalid time {}:{}", self.low_hour, self.low_min);
            return Err(Error::UserInputError("Invalid time".to_string()));
        } else {
            Utc.with_ymd_and_hms(2020, 1, 1, self.low_hour, self.low_min, 0)
                .unwrap()
        };
        // current datetime which is updated as per conditions
        let mut config_date_low_hour = self.get_hms(self.low_hour, self.low_min, self.dt.day())?;
        let mut config_date_high_hour =
            self.get_hms(self.high_hour, self.high_min, self.dt.day())?;

        // check if the current day is configured in the input week range
        if self.dt.weekday().num_days_from_monday() >= self.week_start
            && self.dt.weekday().num_days_from_monday() <= self.week_end
            && complex_high_time > complex_low_time
        {
            info!(
                "config_date_low_hour: {} config_date_high_hour: {} and current local time {} ",
                config_date_low_hour,
                config_date_high_hour,
                self.dt.naive_local()
            );

            let t = Timer {
                dt: self.dt,
                config_date_low_hour,
                config_date_high_hour,
            };
            Ok(t.cmp_time())
        } else if complex_high_time < complex_low_time {
            info!("current rule is for rules whose end time is extending midnight");
            // check if current day has passed the end day of rule
            // for example RULE = Mon-Fri 7AM - 01AM, and its sat 01:10 AM
            if self.dt.weekday().num_days_from_monday() == self.week_end + 1 {
                config_date_low_hour =
                    self.get_hms(self.low_hour, self.low_min, self.dt.day() - 1)?;
                let t = Timer {
                    dt: self.dt,
                    config_date_low_hour,
                    config_date_high_hour,
                };
                return Ok(t.cmp_time());
            }
            // for example RULE = Mon-Fri 7 - 02 AM, and its Mon 1 AM
            else if self.dt.weekday().num_days_from_monday() == self.week_start
                && self.dt.hour() < complex_high_time.hour()
            {
                // downscaling
                return Ok(false);
            } else if self.dt.weekday().num_days_from_monday() >= self.week_start
                && self.dt.weekday().num_days_from_monday() <= self.week_end
            {
                // if current time has crossed 12 AM but less than or equals to high hour
                if self.dt.hour() <= complex_high_time.hour() {
                    //below condition is needed if minutes are involved, for example scale down is 2:30 AM
                    config_date_low_hour =
                        self.get_hms(self.low_hour, self.low_min, self.dt.day() - 1)?;
                    let t = Timer {
                        dt: self.dt,
                        config_date_low_hour,
                        config_date_high_hour,
                    };
                    return Ok(t.cmp_time());
                }
                // if current time is less 12 AM but greater than low hour
                else if self.dt.hour() >= complex_low_time.hour() {
                    // below is needed if there are minutes involved, for example start time is 7:30 AM
                    config_date_high_hour =
                        self.get_hms(self.high_hour, self.high_min, self.dt.day() + 1)?;
                    let t = Timer {
                        dt: self.dt,
                        config_date_low_hour,
                        config_date_high_hour,
                    };
                    return Ok(t.cmp_time());
                }
            }

            Ok(false)
        } else {
            // current day is not configured in the uptime
            debug!("current day is not configured in the uptime,hence downscaling");
            Ok(false)
        }
    }

    fn get_hms(&self, hr: u32, min: u32, day: u32) -> Result<NaiveDateTime, Error> {
        if let Some(ddd) = NaiveDate::from_ymd_opt(self.dt.year(), self.dt.month(), day) {
            if let Some(hms) = ddd.and_hms_milli_opt(hr, min, 0, 0) {
                return Ok(hms);
            } else {
                error!("invalid  time{}:{}", hr, min);
            }
        } else {
            error!(
                "invalid date {}/{}/{}",
                self.dt.year(),
                self.dt.month(),
                day
            )
        }
        Err(Error::UserInputError("Invalid datetime".to_string()))
    }
}

pub fn is_uptime(m: Captures) -> Result<bool, Error> {
    let week_start = current_day(&m[1]);
    let week_end = current_day(&m[2]);
    let low_hour: u32 = FromStr::from_str(&m[3])?;
    let low_min: u32 = FromStr::from_str(&m[4])?;
    let high_hour: u32 = FromStr::from_str(&m[5])?;
    let high_min: u32 = FromStr::from_str(&m[6])?;
    let config_tz: &str = &m["tz"];
    let tz: Tz = config_tz.parse()?;
    // get the current datetime based on the timezone
    let dt: DateTime<Tz> = Utc::now().with_timezone(&tz);

    let upt_chk = UpTimeCheck {
        week_start,
        week_end,
        low_hour,
        low_min,
        high_hour,
        high_min,
        dt,
    };
    upt_chk.is_uptime()
}

#[cfg(test)]
mod timecheck_unit_test {
    use chrono::{NaiveDate, TimeZone};
    use chrono_tz::Australia::Sydney;
    use regex::Regex;
    use std::str::FromStr;

    struct CurrentDateTime {
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        min: u32,
        sec: u32,
    }
    impl CurrentDateTime {
        fn new(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> Self {
            CurrentDateTime {
                year,
                month,
                day,
                hour,
                min,
                sec,
            }
        }
        fn get_data(&self, r: &str) -> UpTimeCheck {
            let v = Regex::new(
                r"^([a-zA-Z]{3})-([a-zA-Z]{3}) (\d\d):(\d\d)-(\d\d):(\d\d) (?P<tz>[a-zA-Z/_]+)$",
            )
            .unwrap();
            let m = v.captures(r).unwrap();

            let nd = NaiveDate::from_ymd(self.year, self.month, self.day)
                .and_hms(self.hour, self.min, self.sec);
            // ALl tests are in sydney timezone but can work with anyother
            let dt = Sydney.from_local_datetime(&nd).unwrap();

            UpTimeCheck {
                week_start: current_day(&m[1]),
                week_end: current_day(&m[2]),
                low_hour: FromStr::from_str(&m[3]).unwrap(),
                low_min: FromStr::from_str(&m[4]).unwrap(),
                high_hour: FromStr::from_str(&m[5]).unwrap(),
                high_min: FromStr::from_str(&m[6]).unwrap(),
                dt,
            }
        }
    }

    use crate::time_check::{current_day, UpTimeCheck};
    #[test]
    // Rule    : Mon-Fri 00:00-23:59 Australia/Sydney
    // Uptime  : Mon(00:00 AM)-Fri(23:59PM)
    // Downtime: Sat(00:01 AM)-Sun(23:59AM)
    fn test_check_all_up_weekdays() {
        let rule = "Mon-Fri 00:00-23:59 Australia/Sydney";
        // Datetime: 29-Aug-2022 Day: Monday Time:01 AM
        // Expected : Resources should be UP
        let mut cdt = CurrentDateTime::new(2022, 08, 29, 01, 00, 00);
        let mut u = cdt.get_data(rule);
        let z = u.is_uptime();
        print!("{z:?}");
        assert_eq!(u.is_uptime().unwrap(), true);
        // Datetime: 30-Aug-2022 Day: Tuesday Time:03 AM
        // Expected : Resources should be UP
        cdt = CurrentDateTime::new(2022, 08, 30, 03, 00, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), true);
        // Datetime: 02-Sep-2022 Day: Friday Time: 23:59 PM
        // Expected : Resources should be UP
        cdt = CurrentDateTime::new(2022, 09, 02, 23, 59, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), true);
        // Datetime: 03-Sep-2022 Day: Saturday Time: 00:00 AM
        // Expected : Resources should be DOWN
        cdt = CurrentDateTime::new(2022, 09, 03, 00, 01, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), false);
        // Datetime: 04-Sep-2022 Day: Sunday Time: 09:00 AM
        // Expected : Resources should be DOWN
        cdt = CurrentDateTime::new(2022, 09, 04, 09, 00, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), false);
        // Datetime: 04-Sep-2022 Day: Sunday Time: 09:00 AM
        // Expected : Resources should be DOWN
        cdt = CurrentDateTime::new(2022, 09, 04, 09, 00, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), false);
    }

    #[test]
    // Rule  :   Mon-Fri 07:00-02:00 Australia/Sydney
    // Uptime:   Weekdays Mon(7AM) to Tue(2AM) | Downtime: Weekdays Mon(2AM) to Mon(7AM)
    //           Weekdays Tue(7AM) to Wed(2AM) |           Weekdays Tue(2AM) to Tue(7AM)
    //           Weekdays Wed(7AM) to Thu(2AM) |           Weekdays Wed(2AM) to Wed(7AM)
    //           Weekdays Thu(7AM) to Fri(2AM) |           Weekdays Thu(2AM) to Thu(7AM)
    //           Weekdays Fri(7AM) to Sat(2AM) |           Weekdays Fri(2AM) to Fri(7AM)
    //                                         |           Weekdays Sat(2AM) to Mon(7AM)
    fn test_check_with_uptime_extending_overnite() {
        let rule = "Mon-Fri 07:00-02:00 Australia/Sydney";
        // Datet: 05-Sep-2022 Day: Monday Time:07:01 AM
        // Expected : Resources should be UP
        let mut cdt = CurrentDateTime::new(2022, 09, 05, 07, 01, 00);
        let mut u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), true);
        // Date: 06-Sep-2022 Day: Tuesday Time:01:00 AM
        // Expected : Resources should be UP
        cdt = CurrentDateTime::new(2022, 09, 06, 9, 00, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), true);
        // Datetime: 09-Sep-2022 Day: Friday Time: 23:59 PM
        // Expected : Resources should be UP
        cdt = CurrentDateTime::new(2022, 09, 09, 23, 59, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), true);
        // Datetime: 10-Sep-2022 Day: Saturday Time: 01:59 AM
        // Expected : Resources should be UP
        cdt = CurrentDateTime::new(2022, 09, 10, 01, 59, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), true);
        // Datetime: 10-Sep-2022 Day: Saturday Time: 02:01 AM
        // Expected : Resources should be DOWN
        cdt = CurrentDateTime::new(2022, 09, 10, 02, 01, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), false);
        // Datetime: 10-Sep-2022 Day: Saturday Time: 09:00 AM
        // Expected : Resources should be DOWN
        cdt = CurrentDateTime::new(2022, 09, 10, 09, 00, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), false);
        // Datetime: 11-Sep-2022 Day: Sunday Time: 09:00 PM
        // Expected : Resources should be DOWN
        cdt = CurrentDateTime::new(2022, 09, 11, 21, 00, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), false);
        // Datetime: 12-Sep-2022 Day: Monday Time: 03:00 AM
        // Expected : Resources should be DOWN
        cdt = CurrentDateTime::new(2022, 09, 12, 03, 00, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), false);
        // Datetime: 12-Sep-2022 Day: Monday Time: 01:00 AM
        // Expected : Resources should be DOWN
        cdt = CurrentDateTime::new(2022, 09, 12, 01, 00, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), false);
    }

    #[test]
    // Rule:     Mon-Fri 07:00-19:00 Australia/Sydney
    // Uptime:   Weekdays Mon(7AM) to Mon(7PM) | Downtime: Weekdays Mon(7PM) to Tue(7AM)
    //           Weekdays Tue(7AM) to Tue(7PM) |           Weekdays Tue(7PM) to Wed(7AM)
    //           Weekdays Wed(7AM) to Wed(7PM) |           Weekdays Wed(7PM) to Thu(7AM)
    //           Weekdays Thu(7AM) to Thu(7PM) |           Weekdays Thu(7PM) to Fri(7AM)
    //           Weekdays Fri(7AM) to Fri(7PM) |           Weekdays Fri(7PM) to Mon(7AM)
    fn test_check_with_uptime_same_day() {
        let rule = "Mon-Fri 07:00-19:00 Australia/Sydney";
        // Datet: 05-Sep-2022 Day: Monday Time:07:01 AM
        // Expected : Resources should be UP
        let mut cdt = CurrentDateTime::new(2022, 09, 05, 07, 01, 00);
        let mut u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), true);
        // Date: 05-Sep-2022 Day: Monday Time:07:01 PM
        // Expected : Resources should be DOWN
        cdt = CurrentDateTime::new(2022, 09, 05, 19, 01, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), false);
        // Date: 06-Sep-2022 Day: Tuesday Time:05:00 AM
        // Expected : Resources should be DOWN
        cdt = CurrentDateTime::new(2022, 09, 06, 05, 01, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), false);
        // Datetime: 03-Sep-2022 Day: Friday Time: 07:59 PM
        // Expected : Resources should be DOWN
        cdt = CurrentDateTime::new(2022, 09, 03, 19, 59, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), false);
        // Datetime: 10-Sep-2022 Day: Saturday Time: 07:01 AM
        // Expected : Resources should be DOWN
        cdt = CurrentDateTime::new(2022, 09, 10, 07, 01, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), false);
        // Datetime: 11-Sep-2022 Day: Sunday Time: 07:01 AM
        // Expected : Resources should be DOWN
        cdt = CurrentDateTime::new(2022, 09, 11, 07, 01, 00);
        u = cdt.get_data(rule);
        assert_eq!(u.is_uptime().unwrap(), false);
    }
}
