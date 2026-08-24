//! Port of `qlogistiki/transaction_line.py`.

use crate::account::ChartOfAccounts;
use crate::utils::{f2gr, round2};

/// One line of a double-entry transaction. The value's sign decides
/// debit (>0) or credit (<0); `delta` reverses the sign for income,
/// liabilities and capital accounts.
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionLine {
    pub account_name: String,
    pub value: f64,
    pub sxolio: String,
}

impl TransactionLine {
    pub fn new(account_name: &str, value: f64, sxolio: &str) -> Self {
        TransactionLine {
            account_name: account_name.to_string(),
            value: round2(value),
            sxolio: sxolio.trim().to_string(),
        }
    }

    pub fn debit(&self) -> f64 {
        if self.value > 0.0 {
            self.value
        } else {
            0.0
        }
    }

    pub fn credit(&self) -> f64 {
        if self.value < 0.0 {
            -self.value
        } else {
            0.0
        }
    }

    pub fn delta(&self, chart: &ChartOfAccounts) -> f64 {
        if chart.is_reverse(&self.account_name) {
            -self.value
        } else {
            self.value
        }
    }

    pub fn mul(&self, number: f64) -> TransactionLine {
        TransactionLine {
            account_name: self.account_name.clone(),
            value: round2(self.value * number),
            sxolio: self.sxolio.clone(),
        }
    }

    /// Addition requires identical account names.
    pub fn checked_add(&self, other: &TransactionLine) -> Result<TransactionLine, String> {
        if self.account_name != other.account_name {
            return Err("For addition accounts must me the same".to_string());
        }
        Ok(TransactionLine {
            account_name: self.account_name.clone(),
            value: self.value + other.value,
            sxolio: self.sxolio.clone(),
        })
    }
}

impl std::fmt::Display for TransactionLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:<30} {:<30} {:>14} {:>14}",
            self.account_name,
            self.sxolio.chars().take(30).collect::<String>(),
            f2gr(self.debit()),
            f2gr(self.credit())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::OMADES_TYPES_GR;

    fn chart() -> ChartOfAccounts {
        ChartOfAccounts::new(
            "gr",
            OMADES_TYPES_GR
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
        )
    }

    #[test]
    fn test_tr001() {
        let tl1 = TransactionLine::new("Aa.Bb.Cc", -100.0, "");
        let tl2 = TransactionLine::new("Aa.Bb.Cc", -100.0, "");
        assert_eq!(tl1, tl2);
        assert_eq!(tl1.debit(), 0.0);
        assert_eq!(tl1.credit(), 100.0);
        assert_eq!(tl1.value, -100.0);
        assert_eq!(tl1.mul(-2.0), TransactionLine::new("Aa.Bb.Cc", 200.0, ""));
        assert_eq!(tl2.mul(1.5), TransactionLine::new("Aa.Bb.Cc", -150.0, ""));
    }

    #[test]
    fn test_delta_reverse() {
        let cht = chart();
        // "70..." is esoda => reversed delta
        let l1 = TransactionLine::new("70.00.001", 50.0, "");
        assert_eq!(l1.delta(&cht), -50.0);
        // "20..." is apothemata => plain delta
        let l2 = TransactionLine::new("20.00.001", 50.0, "");
        assert_eq!(l2.delta(&cht), 50.0);
    }
}
