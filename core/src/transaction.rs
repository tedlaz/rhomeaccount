//! Port of `qlogistiki/transaction.py`.

use chrono::NaiveDate;

use crate::account::ChartOfAccounts;
use crate::transaction_line::TransactionLine;
use crate::utils::round2;

/// A full kartella row (`Trl` namedtuple + running totals from `kartella()`).
#[derive(Debug, Clone)]
pub struct KartellaLine {
    pub id: i32,
    pub date: NaiveDate,
    pub parastatiko: String,
    pub perigrafi: String,
    pub account_name: String,
    pub sxolio: String,
    pub value: f64,
    pub delta: f64,
    pub debit: f64,
    pub credit: f64,
    pub tvalue: f64,
    pub tdebit: f64,
    pub tcredit: f64,
    pub tdelta: f64,
}

/// Class dealing with transactions.
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: i32,
    pub date: NaiveDate,
    pub parastatiko: String,
    pub perigrafi: String,
    pub lines: Vec<TransactionLine>,
}

impl Transaction {
    /// Fails on a date that is not a real ISO date, rather than silently
    /// substituting one — a wrong date silently corrupts every report.
    pub fn try_new(
        iso_date: &str,
        parastatiko: &str,
        perigrafi: &str,
        idv: i32,
    ) -> Result<Self, String> {
        let date = NaiveDate::parse_from_str(iso_date, "%Y-%m-%d")
            .map_err(|_| format!("'{}' is not a valid ISO date", iso_date))?;
        Ok(Transaction {
            id: idv,
            date,
            parastatiko: parastatiko.to_string(),
            perigrafi: perigrafi.to_string(),
            lines: Vec::new(),
        })
    }

    pub fn set_id(&mut self, idv: i32) -> Result<(), String> {
        if self.id != 0 {
            return Err(format!(
                "Transaction {} already has an id, cannot set it to {}",
                self.id, idv
            ));
        }
        self.id = idv;
        Ok(())
    }

    /// Sum of all line values.
    pub fn rest(&self) -> f64 {
        round2(self.lines.iter().map(|l| l.value).sum())
    }

    /// Generate a unique id string.
    pub fn uid(&self) -> String {
        let date_part = self.date.format("%Y%m%d").to_string();
        let parastatiko_part = self.parastatiko.replace(' ', "");
        let val_part = format!("{:.1}", self.total()).replace([',', '.'], "");
        format!("{}{}{}", date_part, parastatiko_part, val_part)
    }

    pub fn is_balanced(&self) -> bool {
        self.lines.len() >= 2 && self.rest() == 0.0
    }

    /// Total debit of the transaction.
    pub fn total(&self) -> f64 {
        self.lines.iter().map(|l| l.debit()).sum()
    }

    pub fn value(&self) -> f64 {
        self.total()
    }

    /// Lines matching an account-name prefix, with a running total.
    pub fn get_lines_by_account(
        &self,
        account_part: &str,
        running_total: &mut f64,
        found: &mut Vec<(i32, NaiveDate, String, String, f64, f64, f64)>,
    ) {
        for line in &self.lines {
            if line.account_name.starts_with(account_part) {
                let per = if !line.sxolio.is_empty() {
                    format!("{}, {}", self.perigrafi, line.sxolio)
                } else {
                    self.perigrafi.clone()
                };
                *running_total += line.value;
                found.push((
                    self.id,
                    self.date,
                    self.parastatiko.clone(),
                    per,
                    line.debit(),
                    line.credit(),
                    round2(*running_total),
                ));
            }
        }
    }

    pub fn add_line(&mut self, account_name: &str, value: f64, sxolio: &str) {
        self.lines.push(TransactionLine {
            account_name: account_name.to_string(),
            value: round2(value),
            sxolio: sxolio.trim().to_string(),
        });
    }

    pub fn add_connected_lines(&mut self, acc1: &str, acc2: &str, value: f64, pososto: f64) {
        self.add_line(acc1, value, "");
        self.add_line(acc2, value * pososto / 100.0, "");
    }

    /// Adds the balancing final line.
    pub fn add_last_line(&mut self, account_name: &str, sxolio: &str) -> Result<(), String> {
        let rest = self.rest();
        if rest == 0.0 {
            return Err("Transaction is already balanced".to_string());
        }
        self.lines.push(TransactionLine {
            account_name: account_name.to_string(),
            value: -rest,
            sxolio: sxolio.trim().to_string(),
        });
        Ok(())
    }

    pub fn last_account(&self) -> Result<&str, String> {
        self.lines
            .last()
            .map(|l| l.account_name.as_str())
            .ok_or_else(|| "Impossible value".to_string())
    }

    pub fn last_delta(&self, chart: &ChartOfAccounts) -> f64 {
        match self.lines.last() {
            None => 0.0,
            Some(l) => l.delta(chart),
        }
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Transaction {
            id: 0,
            date: chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            parastatiko: String::new(),
            perigrafi: String::new(),
            lines: Vec::new(),
        }
    }
}

/// Sorting equivalent of Python `sorted(transactions)`:
/// by date, then by insertion id.
pub fn sort_transactions<'a, I>(transactions: I) -> Vec<&'a Transaction>
where
    I: IntoIterator<Item = &'a Transaction>,
{
    let mut v: Vec<&Transaction> = transactions.into_iter().collect();
    v.sort_by_key(|t| (t.date, t.id));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chart() -> ChartOfAccounts {
        ChartOfAccounts::new(
            "gr",
            [
                ("1", "pagia"),
                ("2", "apothemata"),
                ("3", "apaitiseis"),
                ("4", "kefalaio"),
                ("5", "ypoxreoseis"),
                ("54.00", "fpa"),
                ("6", "ejoda"),
                ("7", "esoda"),
                ("8", "anorgana"),
            ]
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect(),
        )
    }

    #[test]
    fn test_transaction_01() {
        let _cht = chart();
        let mut tr1 = Transaction::try_new("2020-01-10", "", "Σουπερμαρκετ πόπη", 0).unwrap();
        tr1.add_line("20.00.00.024", 100.0, "");
        tr1.add_line("54.00.20.024", 24.0, "");
        tr1.add_last_line("50.00.00.001", "").unwrap();
        assert_eq!(tr1.uid(), "202001101240");
        assert_eq!(tr1.value(), 124.0);
    }

    #[test]
    fn test_transaction_comparison() {
        let _cht = chart();
        let mut tr1 = Transaction::try_new("2020-01-15", "", "Σουπερμαρκετ πόπη", 1).unwrap();
        tr1.add_line("20.00.00.024", 100.0, "");
        tr1.add_line("54.00.20.024", 24.0, "");
        tr1.add_last_line("50.00.00.001", "").unwrap();

        let mut tr2 = Transaction::try_new("2020-01-12", "", "Σουπερμαρκετ πόπη", 2).unwrap();
        tr2.add_line("20.00.00.024", 100.0, "");
        tr2.add_line("54.00.20.024", 24.0, "");
        tr2.add_last_line("50.00.00.001", "").unwrap();

        assert!((tr1.date, tr1.id) > (tr2.date, tr2.id));
    }
}
