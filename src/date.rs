#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    pub fn today() -> Self {
        Self::from_days_since_epoch(days_since_epoch())
    }

    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.split('-');
        let year = parts.next()?.parse().ok()?;
        let month = parts.next()?.parse().ok()?;
        let day = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Self::new(year, month, day)
    }

    pub fn new(year: i32, month: u32, day: u32) -> Option<Self> {
        if !(1..=12).contains(&month) {
            return None;
        }
        if !(1..=days_in_month(year, month)).contains(&day) {
            return None;
        }
        Some(Self { year, month, day })
    }

    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub fn add_days(self, delta: i32) -> Self {
        Self::from_days_since_epoch(self.days_since_epoch() + i64::from(delta))
    }

    pub fn add_months(self, delta: i32) -> Self {
        let month_index = self.year * 12 + self.month as i32 - 1 + delta;
        let year = month_index.div_euclid(12);
        let month = month_index.rem_euclid(12) as u32 + 1;
        let day = self.day.min(days_in_month(year, month));
        Self { year, month, day }
    }

    pub fn weekday_monday0(self) -> u32 {
        (self.days_since_epoch() + 3).rem_euclid(7) as u32
    }

    fn from_days_since_epoch(days: i64) -> Self {
        let mut days = days;
        let mut year = 1970;

        if days >= 0 {
            loop {
                let yd = days_in_year(year) as i64;
                if days < yd {
                    break;
                }
                days -= yd;
                year += 1;
            }
        } else {
            loop {
                year -= 1;
                days += days_in_year(year) as i64;
                if days >= 0 {
                    break;
                }
            }
        }

        let mut month = 1;
        loop {
            let md = days_in_month(year, month) as i64;
            if days < md {
                break;
            }
            days -= md;
            month += 1;
        }

        Self {
            year,
            month,
            day: days as u32 + 1,
        }
    }

    fn days_since_epoch(self) -> i64 {
        let mut days = 0i64;
        if self.year >= 1970 {
            for year in 1970..self.year {
                days += days_in_year(year) as i64;
            }
        } else {
            for year in self.year..1970 {
                days -= days_in_year(year) as i64;
            }
        }
        for month in 1..self.month {
            days += days_in_month(self.year, month) as i64;
        }
        days + i64::from(self.day - 1)
    }
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

pub fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "",
    }
}

pub fn is_overdue(due_date: &str) -> bool {
    Date::parse(due_date).is_some_and(|date| date <= Date::today())
}

pub fn days_until(due_date: &str) -> Option<i64> {
    let date = Date::parse(due_date)?;
    Some(date.days_since_epoch() - Date::today().days_since_epoch())
}

fn days_since_epoch() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    (now + local_offset_seconds()).div_euclid(86_400)
}

fn local_offset_seconds() -> i64 {
    use std::sync::OnceLock;
    static OFFSET: OnceLock<i64> = OnceLock::new();
    *OFFSET.get_or_init(|| {
        std::process::Command::new("date")
            .arg("+%z")
            .output()
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .and_then(|s| parse_tz_offset(s.trim()))
            .unwrap_or(0)
    })
}

fn parse_tz_offset(s: &str) -> Option<i64> {
    let bytes = s.as_bytes();
    if bytes.len() != 5 {
        return None;
    }
    let sign: i64 = match bytes[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hours: i64 = s.get(1..3)?.parse().ok()?;
    let minutes: i64 = s.get(3..5)?.parse().ok()?;
    Some(sign * (hours * 3600 + minutes * 60))
}

fn days_in_year(year: i32) -> u32 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::{days_in_month, days_until, Date};

    #[test]
    fn parses_valid_iso_dates() {
        assert_eq!(
            Date::parse("2026-06-25"),
            Some(Date {
                year: 2026,
                month: 6,
                day: 25
            })
        );
    }

    #[test]
    fn rejects_invalid_dates() {
        assert_eq!(Date::parse("2026-02-31"), None);
        assert_eq!(Date::parse("2026-13-01"), None);
        assert_eq!(Date::parse("not-a-date"), None);
    }

    #[test]
    fn handles_leap_year_month_lengths() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
    }

    #[test]
    fn moves_across_month_boundaries() {
        let date = Date::parse("2026-03-01").unwrap();
        assert_eq!(date.add_days(-1).iso(), "2026-02-28");
        assert_eq!(date.add_days(1).iso(), "2026-03-02");
    }

    #[test]
    fn clamps_day_when_moving_months() {
        let date = Date::parse("2026-01-31").unwrap();
        assert_eq!(date.add_months(1).iso(), "2026-02-28");
    }

    #[test]
    fn reports_days_until_date() {
        let today = Date::today();
        assert_eq!(days_until(&today.iso()), Some(0));
        assert_eq!(days_until(&today.add_days(3).iso()), Some(3));
        assert_eq!(days_until(&today.add_days(-1).iso()), Some(-1));
    }
}
