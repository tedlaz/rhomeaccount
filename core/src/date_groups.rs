//! Port of `qlogistiki/date_groups.py`.
//!
//! Maps dates to text keys used for time-series grouping.

use chrono::{Datelike, NaiveDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grouping {
    YearMonth,
    Year,
    Trimino,
    Tetramino,
    Ejamino,
}

impl Grouping {
    pub fn group(self, dat: NaiveDate) -> String {
        match self {
            Grouping::YearMonth => date2year_month(dat),
            Grouping::Year => date2year(dat),
            Grouping::Trimino => date2trimino(dat),
            Grouping::Tetramino => date2tetramino(dat),
            Grouping::Ejamino => date2ejamino(dat),
        }
    }
}

/// "2021-12-16" -> "20211216"
pub fn date2date(dat: NaiveDate) -> String {
    dat.format("%Y%m%d").to_string()
}

/// "2021-12-16" -> "2112"
pub fn date2year_month(dat: NaiveDate) -> String {
    format!("{:02}{:02}", dat.year() % 100, dat.month())
}

/// "2021-12-16" -> "2021"
pub fn date2year(dat: NaiveDate) -> String {
    format!("{}", dat.year())
}

fn quarter(dat: NaiveDate) -> u32 {
    (dat.month() - 1) / 3 + 1
}

/// "2021-12-16" -> "214"  (year 21, quarter 4)
pub fn date2trimino(dat: NaiveDate) -> String {
    format!("{:02}{}", dat.year() % 100, quarter(dat))
}

fn fourmonth(dat: NaiveDate) -> u32 {
    (dat.month() - 1) / 4 + 1
}

/// "2021-12-16" -> "213"  (year 21, third 4-month period)
pub fn date2tetramino(dat: NaiveDate) -> String {
    format!("{:02}{}", dat.year() % 100, fourmonth(dat))
}

fn halfyear(dat: NaiveDate) -> u32 {
    if dat.month() <= 6 {
        1
    } else {
        2
    }
}

/// "2021-12-16" -> "212"  (year 21, half-year 2)
pub fn date2ejamino(dat: NaiveDate) -> String {
    format!("{:02}{}", dat.year() % 100, halfyear(dat))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn test_groups() {
        assert_eq!(date2date(d("2021-12-16")), "20211216");
        assert_eq!(date2year_month(d("2021-12-16")), "2112");
        assert_eq!(date2year(d("2021-12-16")), "2021");
        assert_eq!(date2trimino(d("2021-12-16")), "214");
        assert_eq!(date2trimino(d("2021-01-05")), "211");
        assert_eq!(date2tetramino(d("2021-05-05")), "212");
        assert_eq!(date2ejamino(d("2021-07-05")), "212");
        assert_eq!(date2ejamino(d("2021-06-30")), "211");
    }
}
