//! Port of `qlogistiki/parser_text.py`.
//!
//! Parses a book folder containing a `000` metadata file plus journal files.
//! Journal first line must be `j-open`, `j-normal` or `j-close`; header lines
//! start with an ISO date, detail lines start with two spaces, values are
//! Greek-formatted numbers, `#` starts a comment, `@` is a validation line.

use crate::account::ChartOfAccounts;
use crate::book::Book;
use crate::transaction::Transaction;
use crate::utils::gr2float;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const BOOK_METADATA_FILE: &str = "000";

struct ParserState {
    trn_id: i32,
}

/// Matches the shape `dddd-dd-dd` only — whether it is a *real* date is a
/// separate question, so that `2022-13-45` is reported as a bad date instead
/// of being silently skipped as "not a header line".
fn starts_with_iso_date(line: &str) -> bool {
    let bytes = line.as_bytes();
    if bytes.len() < 10 {
        return false;
    }
    bytes[..10].iter().enumerate().all(|(i, b)| match i {
        4 | 7 => *b == b'-',
        _ => b.is_ascii_digit(),
    })
}

fn starts_with_two_spaces(line: &str) -> bool {
    // Python regex was r"^  ." — two spaces followed by any character.
    line.starts_with("  ") && line.len() >= 3
}

fn extract_braces(line: &str) -> Option<String> {
    let start = line.find('{')?;
    let end = line.find('}')?;
    if end > start {
        Some(line[start + 1..end].to_string())
    } else {
        None
    }
}

/// Reads the `000` metadata file: returns (valid accounts, (omada, typos) pairs, book name).
pub fn parse_metadata(
    book_dir: &Path,
) -> Result<(Vec<String>, Vec<(String, String)>, String), String> {
    let path000 = book_dir.join(BOOK_METADATA_FILE);
    let content = fs::read_to_string(&path000)
        .map_err(|e| format!("Cannot read {}: {}", path000.display(), e))?;

    let mut book_name = String::new();
    let mut omades: Vec<(String, String)> = Vec::new();
    let mut valid_accounts: Vec<String> = Vec::new();

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix('@') {
            book_name = rest.split_whitespace().collect::<Vec<_>>().join(" ");
        }
        if let Some(rest) = line.strip_prefix('+') {
            if let Some(account) = rest.split_whitespace().next() {
                valid_accounts.push(account.to_string());
            }
        }
        if let Some(rest) = line.strip_prefix('>') {
            let tokens: Vec<&str> = rest.split_whitespace().collect();
            if tokens.len() >= 2 {
                // Python keeps these in a dict: a repeated prefix overwrites
                // in place rather than adding a second category.
                match omades.iter_mut().find(|(prefix, _)| prefix == tokens[0]) {
                    Some(entry) => entry.1 = tokens[1].to_string(),
                    None => omades.push((tokens[0].to_string(), tokens[1].to_string())),
                }
            }
        }
    }
    Ok((valid_accounts, omades, book_name))
}

fn parse_header_line(line: &str) -> (String, String, String) {
    let mut parts = line.splitn(2, char::is_whitespace);
    let dat = parts.next().unwrap_or("").trim().to_string();
    let fper_full = parts.next().unwrap_or("").to_string();

    let par = extract_braces(&fper_full);
    let fper = match &par {
        Some(p) => fper_full.replace(&format!("{{{}}}", p), ""),
        None => fper_full.clone(),
    };
    (
        dat,
        fper.trim().to_string(),
        par.unwrap_or_default().trim().to_string(),
    )
}

/// The value part of a detail line.
#[derive(Debug, Clone, PartialEq)]
pub enum DetailValue {
    /// No value written — this line balances the transaction.
    Balancing,
    /// An explicit value, including an explicit zero.
    Value(f64),
    /// A value was written but is not a number; carries the offending token.
    Unparseable(String),
}

/// Returns (sxolio, account, value).
///
/// A missing value and an unreadable value are kept apart on purpose: the
/// first means "balance this transaction", the second is a typo that must be
/// reported. Collapsing them lets a mistyped amount be silently absorbed into
/// the balancing line, producing a book that balances and is wrong.
fn parse_detail_line(line: &str) -> (String, String, DetailValue) {
    let mut split_iter = line.split('#');
    let accval = split_iter.next().unwrap_or("");
    let sxolio = split_iter
        .next()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let mut tokens = accval.split_whitespace();
    let account = tokens.next().unwrap_or("").to_string();
    let val = match tokens.next() {
        None => DetailValue::Balancing,
        Some(tok) => match gr2float(tok) {
            Some(v) => DetailValue::Value(v),
            None => DetailValue::Unparseable(tok.to_string()),
        },
    };
    (sxolio, account, val)
}

fn is_line_to_ignore(row_line: &str) -> bool {
    let line = row_line.trim();
    line.len() < 4 || line.starts_with('#')
}

fn parse_imerologio(
    book: &mut Book,
    declared_accounts: &HashSet<String>,
    errors: &mut Vec<String>,
    filepath: &Path,
) {
    let content = match fs::read_to_string(filepath) {
        Ok(c) => c,
        Err(e) => {
            errors.push(format!("Cannot read {}: {}", filepath.display(), e));
            return;
        }
    };
    let mut lines_iter = content.lines();
    let first_line = lines_iter.next().unwrap_or("").trim().to_string();

    if !["j-open", "j-normal", "j-close"].contains(&first_line.as_str()) {
        errors.push(format!(
            "File {} is not compatible. First line: '{}'",
            filepath.display(),
            first_line
        ));
        return;
    }

    let mut state = ParserState { trn_id: 0 };

    for (i, row_line) in lines_iter.enumerate() {
        if is_line_to_ignore(row_line) {
            continue;
        }

        // validation line: @ 2020-05-10 Λογαριασμός -120,32
        let trimmed = row_line.trim();
        if trimmed.starts_with('@') {
            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            if tokens.len() != 4 {
                errors.push(format!(
                    "{}:{}: γραμμή ελέγχου '@' θέλει 4 πεδία (@ ημερομηνία λογαριασμός ποσό), βρέθηκαν {}",
                    filepath.display(),
                    i + 2,
                    tokens.len()
                ));
                continue;
            }
            match gr2float(tokens[3]) {
                Some(cval) => {
                    book.add_validation((tokens[1].to_string(), tokens[2].to_string(), cval))
                }
                None => errors.push(format!(
                    "{}:{}: το ποσό ελέγχου '{}' δεν είναι αριθμός",
                    filepath.display(),
                    i + 2,
                    tokens[3]
                )),
            }
            continue;
        }

        // header line
        if starts_with_iso_date(trimmed) {
            let (dat, perigrafi, parastatiko) = parse_header_line(trimmed);
            match Transaction::try_new(&dat, &parastatiko, &perigrafi, 0)
                .and_then(|trn| book.add_transaction(trn))
            {
                Ok(id) => state.trn_id = id,
                Err(e) => {
                    // Leave trn_id pointing at nothing so the following detail
                    // lines are reported rather than attached to the previous
                    // transaction.
                    state.trn_id = 0;
                    errors.push(format!("{}:{}: {}", filepath.display(), i + 2, e));
                }
            }
            continue;
        }

        // detail line
        if starts_with_two_spaces(row_line) {
            let line = row_line.trim();
            let (sxolio, account, val) = parse_detail_line(line);

            // Checked against the accounts declared in `000`, not against the
            // chart: `add_account` below inserts the account, so a chart check
            // would report only the first occurrence of an unregistered one.
            if !declared_accounts.contains(&account) {
                errors.push(format!(
                    "{}:{}, ο λογαριασμός '{}' δεν είναι καταχωρημένος",
                    filepath.display(),
                    i + 2,
                    account
                ));
            }
            if let Err(e) = book.chart.add_account(&account) {
                errors.push(format!("{}:{}: {}", filepath.display(), i + 2, e));
            }

            let value = match val {
                DetailValue::Unparseable(tok) => {
                    // Skip the line rather than treat it as balancing: the
                    // transaction is then reported as unbalanced instead of
                    // quietly absorbing the typo.
                    errors.push(format!(
                        "{}:{}: το ποσό '{}' δεν είναι αριθμός",
                        filepath.display(),
                        i + 2,
                        tok
                    ));
                    continue;
                }
                other => other,
            };

            match book.get_transaction_mut(state.trn_id) {
                None => errors.push(format!(
                    "{}:{}: η γραμμή '{}' δεν ανήκει σε κάποιο άρθρο",
                    filepath.display(),
                    i + 2,
                    account
                )),
                Some(trn) => match value {
                    DetailValue::Value(v) => trn.add_line(&account, v, &sxolio),
                    DetailValue::Balancing => {
                        if let Err(e) = trn.add_last_line(&account, &sxolio) {
                            errors.push(format!("{}:{}: {}", filepath.display(), i + 2, e));
                        }
                    }
                    DetailValue::Unparseable(_) => unreachable!("handled above"),
                },
            }
        }
    }
}

fn sorted_book_files(filenames: Vec<String>, book_dir: &Path) -> Vec<PathBuf> {
    let mut names: Vec<String> = filenames
        .into_iter()
        .filter(|f| f != BOOK_METADATA_FILE)
        .collect();
    names.sort();
    names.iter().map(|f| book_dir.join(f)).collect()
}

fn is_path_valid_book_data(path_file_list: &[String]) -> bool {
    path_file_list.iter().any(|f| f == BOOK_METADATA_FILE)
}

/// Parses a whole book folder. On success returns the book and a list of
/// non-fatal parsing errors (same as the Python version).
pub fn parse_folder(book_dir: &str) -> Result<(Book, Vec<String>), String> {
    let dir = Path::new(book_dir);
    let filenames: Vec<String> = fs::read_dir(dir)
        .map_err(|_| format!("{} is not valid book path", book_dir))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    if !is_path_valid_book_data(&filenames) {
        return Err(format!("{} is not valid book path", book_dir));
    }

    let (accounts, acc_types, bname) = parse_metadata(dir)?;
    let declared_accounts: HashSet<String> = accounts.iter().cloned().collect();
    let lsxedio = ChartOfAccounts::full(&bname, acc_types, &accounts)?;

    let mut book = Book::new(&bname, lsxedio);

    let filepaths = sorted_book_files(filenames, dir);

    let mut errors = Vec::new();
    for filename in filepaths {
        if filename.is_file() {
            parse_imerologio(&mut book, &declared_accounts, &mut errors, &filename);
        }
    }

    Ok((book, errors))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/book01")
    }

    #[test]
    fn test_parse() {
        let dir = fixture_dir();
        let (mybook, errors) = parse_folder(dir.to_str().unwrap()).unwrap();
        assert!(errors.is_empty(), "errors: {:?}", errors);
        assert!(mybook.is_balanced().is_ok());
        assert!(mybook.get_transaction(13).is_some());
    }

    #[test]
    fn test_invalid_folder() {
        assert!(parse_folder("C:/definitely/not/a/book").is_err());
    }

    /// Copies the fixture book into a temp dir so a journal can be perturbed.
    fn book_with_extra_journal(tag: &str, contents: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qhomeacc_test_{}", tag));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::copy(fixture_dir().join("000"), dir.join("000")).unwrap();
        fs::write(dir.join("202204"), contents).unwrap();
        dir
    }

    fn parse_extra(tag: &str, contents: &str) -> (Book, Vec<String>) {
        let dir = book_with_extra_journal(tag, contents);
        parse_folder(dir.to_str().unwrap()).unwrap()
    }

    #[test]
    fn test_unparseable_value_is_reported_not_absorbed() {
        let (book, errors) = parse_extra(
            "bad_value",
            "j-normal\n\n2022-04-01 Δοκιμή\n  Ταμείο.Μετρητά     12,,50\n  Εσοδα.Κοινόχρηστα\n",
        );
        assert!(
            errors.iter().any(|e| e.contains("12,,50")),
            "errors: {:?}",
            errors
        );
        // The typo must not be silently absorbed into the balancing line.
        assert!(book.is_balanced().is_err(), "errors: {:?}", errors);
    }

    #[test]
    fn test_explicit_zero_is_a_value_not_a_balancing_line() {
        let (book, _) = parse_extra(
            "zero_value",
            "j-normal\n\n2022-04-01 Δοκιμή\n  Ταμείο.Μετρητά     0\n  Εσοδα.Κοινόχρηστα     10\n  Χρεώστες.1-Νικ\n",
        );
        let trn = book.transactions.values().last().unwrap();
        assert_eq!(trn.lines.len(), 3);
        assert_eq!(trn.lines[0].value, 0.0);
        // the balancing line is the last one, not the explicit zero
        assert_eq!(trn.lines[2].value, -10.0);
    }

    #[test]
    fn test_malformed_validation_line_is_reported() {
        let (book, errors) = parse_extra(
            "bad_validation",
            "j-normal\n\n@ 2022-04-01 Ταμείο.Μετρητά χχχ\n",
        );
        assert!(
            errors.iter().any(|e| e.contains("χχχ")),
            "errors: {:?}",
            errors
        );
        assert!(book.validations.is_empty());
    }

    #[test]
    fn test_unregistered_account_reported_every_time() {
        let (_, errors) = parse_extra(
            "unregistered",
            "j-normal\n\n2022-04-01 Δοκιμή\n  Εξοδα.Άγνωστο     10\n  Ταμείο.Μετρητά\n\n2022-04-02 Δοκιμή\n  Εξοδα.Άγνωστο     20\n  Ταμείο.Μετρητά\n",
        );
        let n = errors
            .iter()
            .filter(|e| e.contains("Εξοδα.Άγνωστο") && e.contains("δεν είναι καταχωρημένος"))
            .count();
        assert_eq!(n, 2, "errors: {:?}", errors);
    }

    #[test]
    fn test_bad_header_date_is_reported() {
        let (book, errors) = parse_extra(
            "bad_date",
            "j-normal\n\n2022-13-45 Δοκιμή\n  Ταμείο.Μετρητά     10\n  Εσοδα.Κοινόχρηστα\n",
        );
        assert!(
            errors.iter().any(|e| e.contains("2022-13-45")),
            "errors: {:?}",
            errors
        );
        // no transaction was created, so nothing silently landed on 1970-01-01
        assert!(book.transactions.is_empty(), "errors: {:?}", errors);
    }

    #[test]
    fn test_parse_metadata() {
        let (accounts, omades, name) = parse_metadata(&fixture_dir()).unwrap();
        assert_eq!(name, "test book");
        assert!(accounts.contains(&"Ταμείο.Μετρητά".to_string()));
        assert!(omades.contains(&("Πάγια".to_string(), "pagia".to_string())));
        assert!(omades.contains(&("54.00".to_string(), "fpa".to_string())));
    }
}
