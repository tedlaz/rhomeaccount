//! Port of `qlogistiki/utils.py`: Greek number formatting/parsing, Greek
//! uppercase folding, AFM validation and date helpers.

use chrono::{Datelike, NaiveDate};

/// Round to 2 decimals (equivalent of Python `round(x, 2)` for our data range).
pub fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Transforms a string to uppercase, special for Greek comparison (`grup`).
pub fn grup(txtval: &str) -> String {
    let mapped: String = txtval
        .chars()
        .map(|c| match c {
            'Ά' | 'ά' => 'Α',
            'Έ' | 'έ' => 'Ε',
            'Ή' | 'ή' => 'Η',
            'Ί' | 'ΐ' | 'Ϊ' | 'ί' | 'ϊ' => 'Ι',
            'Ό' | 'ό' => 'Ο',
            'Ύ' | 'Ϋ' | 'ΰ' | 'ϋ' | 'ύ' => 'Υ',
            'Ώ' | 'ώ' => 'Ω',
            _ => c,
        })
        .collect();
    mapped.to_uppercase()
}

/// Greek number text ("1.234,56") to plain text decimal ("1234.56").
pub fn gr2strdec(greek_number: &str) -> String {
    greek_number.replace('.', "").replace(',', ".")
}

/// Greek number text to f64. Returns `None` if parsing fails.
pub fn gr2float(greek_number: &str) -> Option<f64> {
    gr2strdec(greek_number).parse::<f64>().ok()
}

/// Capitalize every dot-separated level of an account name.
pub fn fix_account(account: &str, separator: &str) -> String {
    account
        .split('.')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(separator)
}

/// Algorithmic validation of Greek VAT numbers.
pub fn is_afm(a: &str) -> bool {
    if a.starts_with("00000") || a.len() != 9 || !a.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let digits: Vec<u32> = a.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let d = |i: usize| digits[i];
    let b = d(0) * 256
        + d(1) * 128
        + d(2) * 64
        + d(3) * 32
        + d(4) * 16
        + d(5) * 8
        + d(6) * 4
        + d(7) * 2;
    let c = b % 11;
    let dd = c % 10;
    dd == d(8)
}

/// Hierarchical prefixes of an account name: "a.b.c" -> ["a", "a.b", "a.b.c"].
pub fn account_tree(account: &str, reversed: bool, splitter: &str) -> Vec<String> {
    let spl: Vec<&str> = account.split(splitter).collect();
    let lvls: Vec<String> = (0..spl.len()).map(|i| spl[..=i].join(splitter)).collect();
    if reversed {
        let mut lvls = lvls;
        lvls.reverse();
        lvls
    } else {
        lvls
    }
}

/// Format like Python `gr_num`: "1.010,34", zero becomes `"0   "`,
/// trailing zero decimals are replaced by spaces.
pub fn gr_num(number: f64) -> String {
    if !number.is_finite() {
        return "0   ".to_string();
    }
    let rounded = round2(number);
    let neg = rounded < 0.0;
    let abs = rounded.abs();
    let int_part = abs.trunc() as i64;
    let dec_part = ((abs - int_part as f64) * 100.0).round() as u64;
    let ints = int_part.to_string();
    let mut grouped = String::new();
    let n = ints.len();
    for (i, ch) in ints.chars().enumerate() {
        if i > 0 && (n - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    let dchars: Vec<char> = format!("{:02}", dec_part).chars().collect();
    let (mut d0, mut d1) = (dchars[0], dchars[1]);
    let mut coma = ',';
    if d1 == '0' {
        d1 = ' ';
        if d0 == '0' {
            d0 = ' ';
            coma = ' ';
        }
    }
    let sign = if neg { "-" } else { "" };
    format!("{}{}{}{}{}", sign, grouped, coma, d0, d1)
}

/// Format a number in Greek style: `1.234,56`; zero yields the empty string.
pub fn f2gr(number: f64) -> String {
    if number == 0.0 {
        return String::new();
    }
    let rounded = round2(number);
    let neg = rounded < 0.0;
    let abs = rounded.abs();
    let int_part = abs.trunc() as u64;
    let dec_part = ((abs - int_part as f64) * 100.0).round() as u64;
    let ints = int_part.to_string();
    let mut grouped = String::new();
    let n = ints.len();
    for (i, ch) in ints.chars().enumerate() {
        if i > 0 && (n - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    let sign = if neg { "-" } else { "" };
    format!("{}{},{:02}", sign, grouped, dec_part)
}

/// All dates from `dapo` to `deos` inclusive.
pub fn days_list(dapo: NaiveDate, deos: NaiveDate) -> Vec<NaiveDate> {
    let mut dlist = Vec::new();
    let mut day = dapo;
    while day <= deos {
        dlist.push(day);
        day += chrono::Duration::days(1);
    }
    dlist
}

/// All "YYYYMM" strings between year/month ranges inclusive.
pub fn months_between(yapo: i32, mapo: u32, yeos: i32, meos: u32) -> Vec<String> {
    let mut flist = Vec::new();
    for y in yapo..=yeos {
        for m in 1..=12u32 {
            if y == yapo && m < mapo {
                continue;
            }
            if y == yeos && m > meos {
                continue;
            }
            flist.push(format!("{}{:02}", y, m));
        }
    }
    flist
}

/// All "YYYYMM" strings between two "YYYYMM" strings inclusive.
pub fn months_between_ym(ymapo: &str, ymeos: &str) -> Vec<String> {
    let yapo: i32 = ymapo[..4].parse().unwrap_or(0);
    let mapo: u32 = ymapo[4..].parse().unwrap_or(1);
    let yeos: i32 = ymeos[..4].parse().unwrap_or(0);
    let meos: u32 = ymeos[4..].parse().unwrap_or(1);
    months_between(yapo, mapo, yeos, meos)
}

/// "2023-05-17" -> "202305"
pub fn isodate2ym(isodate: &str) -> String {
    isodate.replace('-', "").get(..6).unwrap_or("").to_string()
}

/// ISO date to integer YYYYMMDD.
pub fn date2int(dat: NaiveDate) -> i32 {
    dat.year() * 10000 + dat.month() as i32 * 100 + dat.day() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gr_num() {
        assert_eq!(gr_num(1010.34), "1.010,34");
        assert_eq!(gr_num(0.0), "0   ");
        assert_eq!(gr_num(123123.50), "123.123,5 ");
        assert_eq!(gr_num(-123123.50), "-123.123,5 ");
        assert_eq!(gr_num(0.01), "0,01");
        assert_eq!(gr_num(-0.01), "-0,01");
        assert_eq!(gr_num(0.10), "0,1 ");
        assert_eq!(gr_num(-82.00), "-82   ");
        assert_eq!(gr_num(f64::NAN), "0   ");
    }

    #[test]
    fn test_account_tree() {
        assert_eq!(
            account_tree("a.b.c", false, "."),
            vec!["a".to_string(), "a.b".to_string(), "a.b.c".to_string()]
        );
        assert_eq!(account_tree("a", false, "."), vec!["a".to_string()]);
        assert_eq!(account_tree("", false, "."), vec!["".to_string()]);
        assert_eq!(
            account_tree("a.b", true, "."),
            vec!["a.b".to_string(), "a".to_string()]
        );
    }

    #[test]
    fn test_grup() {
        assert_eq!(grup("ΐέάό"), "ΙΕΑΟ");
        assert_eq!(grup("Ίώνάς"), "ΙΩΝΑΣ");
        assert_eq!(grup("ϊίΫώrock123"), "ΙΙΥΩROCK123");
    }

    #[test]
    fn test_is_afm() {
        assert!(is_afm("094025817"));
        assert!(!is_afm("094025818"));
        assert!(!is_afm("0"));
        assert!(!is_afm("0940258179"));
    }

    #[test]
    fn test_f2gr() {
        assert_eq!(f2gr(1010.34), "1.010,34");
        assert_eq!(f2gr(0.0), "");
        assert_eq!(f2gr(-82.0), "-82,00");
        assert_eq!(f2gr(1234567.891), "1.234.567,89");
    }

    #[test]
    fn test_months_between_ym() {
        assert_eq!(
            months_between_ym("202112", "202203"),
            vec!["202112", "202201", "202202", "202203"]
        );
    }

    #[test]
    fn test_isodate2ym() {
        assert_eq!(isodate2ym("2023-05-17"), "202305");
    }
}
