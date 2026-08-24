//! Port of `qlogistiki/book.py`.

use std::collections::BTreeMap;

use chrono::{NaiveDate, Utc};

use crate::account::ChartOfAccounts;
use crate::date_groups::Grouping;
use crate::transaction::{sort_transactions, KartellaLine, Transaction};
use crate::utils::{days_list, f2gr, isodate2ym, months_between_ym, round2};

/// Totals for one account in the trial balance.
#[derive(Debug, Clone, Copy, Default)]
pub struct IsoTotals {
    pub tvalue: f64,
    pub tdebit: f64,
    pub tcredit: f64,
    pub tdelta: f64,
}

pub struct Book {
    pub name: String,
    pub chart: ChartOfAccounts,
    /// {id: Transaction}
    pub transactions: BTreeMap<i32, Transaction>,
    pub validations: Vec<(String, String, f64)>,
    last_id: i32,
}

impl Book {
    pub fn new(name: &str, lsx: ChartOfAccounts) -> Self {
        Book {
            name: name.to_string(),
            chart: lsx,
            transactions: BTreeMap::new(),
            validations: Vec::new(),
            last_id: 0,
        }
    }

    pub fn transactions_filter<'a>(
        &'a self,
        apo: Option<&str>,
        eos: Option<&str>,
    ) -> impl Iterator<Item = &'a Transaction> {
        let apo_date = apo.and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
        let eos_date = eos.and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
        self.transactions.values().filter(move |trn| {
            if let Some(apo) = apo_date {
                if trn.date < apo {
                    return false;
                }
            }
            if let Some(eos) = eos_date {
                if trn.date > eos {
                    return false;
                }
            }
            true
        })
    }

    /// Balance of an account (prefix match) up to `eos`.
    pub fn ypoloipo(&self, account_name: &str, eos: Option<&str>) -> f64 {
        let mut ypol = 0.0;
        for trn in self.transactions_filter(None, eos) {
            for line in &trn.lines {
                if line.account_name.starts_with(account_name) {
                    ypol += line.value;
                }
            }
        }
        round2(ypol)
    }

    /// Monthly aggregation: Vec of (ym, running total, month delta).
    pub fn monthly_aggregation(
        &self,
        account_name: &str,
        eos: Option<&str>,
    ) -> Vec<(String, f64, f64)> {
        let mut ymv: BTreeMap<String, f64> = BTreeMap::new();
        for trn in self.transactions_filter(None, eos) {
            let year_month = trn.date.format("%Y%m").to_string();
            for line in &trn.lines {
                if line.account_name.starts_with(account_name) {
                    *ymv.entry(year_month.clone()).or_insert(0.0) += line.delta(&self.chart);
                }
            }
        }
        for v in ymv.values_mut() {
            *v = round2(*v);
        }
        if ymv.is_empty() {
            return Vec::new();
        }
        let apon = ymv.keys().next().unwrap().clone();
        let mut eosn = ymv.keys().last().unwrap().clone();

        if self.ypoloipo(account_name, eos) != 0.0 {
            eosn = isodate2ym(&Utc::now().format("%Y-%m-%d").to_string());
            if let Some(eos) = eos {
                eosn = isodate2ym(eos);
            }
        }

        let ym_list = months_between_ym(&apon, &eosn);
        let mut total = 0.0;
        let mut final_list = Vec::new();
        for ym in ym_list {
            total += round2(*ymv.get(&ym).unwrap_or(&0.0));
            final_list.push((ym.clone(), total, *ymv.get(&ym).unwrap_or(&0.0)));
        }
        final_list
    }

    pub fn acclines(
        &self,
        account_name: &str,
        apo: Option<&str>,
        eos: Option<&str>,
    ) -> Vec<(
        i32,
        NaiveDate,
        String,
        String,
        String,
        String,
        f64,
        f64,
        f64,
        f64,
    )> {
        // (id, date, parastatiko, perigrafi, account, sxolio, value, delta, debit, credit)
        let strans = sort_transactions(self.transactions_filter(apo, eos));
        let mut res = Vec::new();
        for trn in strans {
            for line in &trn.lines {
                if line.account_name.starts_with(account_name) {
                    res.push((
                        trn.id,
                        trn.date,
                        trn.parastatiko.clone(),
                        trn.perigrafi.clone(),
                        line.account_name.clone(),
                        line.sxolio.clone(),
                        line.value,
                        line.delta(&self.chart),
                        line.debit(),
                        line.credit(),
                    ));
                }
            }
        }
        res
    }

    /// Time series grouped by a `Grouping` function:
    /// Vec of (group-key, running total, group delta).
    pub fn time_series(
        &self,
        account_name: &str,
        groupfn: Grouping,
        apo: Option<&str>,
        eos: Option<&str>,
    ) -> Option<Vec<(String, f64, f64)>> {
        let res = self.acclines(account_name, apo, eos);
        if res.is_empty() {
            return None;
        }
        let first_date = res[0].1;
        let last_date = res.last().unwrap().1;

        let mut ldir: BTreeMap<String, f64> = BTreeMap::new();
        let mut sdir: BTreeMap<String, f64> = BTreeMap::new();
        for line in &res {
            let date_a = groupfn.group(line.1);
            *ldir.entry(date_a.clone()).or_insert(0.0) += line.7;
            let e = sdir.entry(date_a).or_insert(0.0);
            *e = round2(*e + line.7);
        }

        let flist = days_list(first_date, last_date);
        let group_set: std::collections::BTreeSet<String> =
            flist.iter().map(|d| groupfn.group(*d)).collect();
        let dlist: Vec<String> = group_set.into_iter().collect();

        let mut total = 0.0;
        let mut final_list = Vec::new();
        for day in dlist {
            total += round2(*ldir.get(&day).unwrap_or(&0.0));
            final_list.push((day.clone(), total, *sdir.get(&day).unwrap_or(&0.0)));
        }
        Some(final_list)
    }

    /// Account card (kartella) with running totals.
    pub fn kartella(
        &self,
        account_name: &str,
        apo: Option<&str>,
        eos: Option<&str>,
    ) -> Vec<KartellaLine> {
        let (mut tvalue, mut tdebit, mut tcredit, mut tdelta) = (0.0, 0.0, 0.0, 0.0);
        let mut lines = Vec::new();

        let transactions = sort_transactions(self.transactions_filter(apo, eos));

        for trn in transactions {
            for line in &trn.lines {
                if !line.account_name.starts_with(account_name) {
                    continue;
                }
                tvalue += line.value;
                tdebit += line.debit();
                tcredit += line.credit();
                tdelta += line.delta(&self.chart);
                lines.push(KartellaLine {
                    id: trn.id,
                    date: trn.date,
                    parastatiko: trn.parastatiko.clone(),
                    perigrafi: trn.perigrafi.clone(),
                    account_name: line.account_name.clone(),
                    sxolio: line.sxolio.clone(),
                    value: line.value,
                    delta: line.delta(&self.chart),
                    debit: line.debit(),
                    credit: line.credit(),
                    tvalue: round2(tvalue),
                    tdebit: round2(tdebit),
                    tcredit: round2(tcredit),
                    tdelta: round2(tdelta),
                });
            }
        }
        lines
    }

    /// Kartella rows ready for display; data rows are reversed like the Python model.
    pub fn model_kartella(
        &self,
        account_name: &str,
        apo: Option<&str>,
        eos: Option<&str>,
    ) -> Vec<[String; 9]> {
        let lines = self.kartella(account_name, apo, eos);
        let mut data: Vec<[String; 9]> = lines
            .iter()
            .map(|lin| {
                [
                    lin.id.to_string(),
                    lin.date.format("%Y-%m-%d").to_string(),
                    f2gr(lin.debit),
                    f2gr(lin.credit),
                    f2gr(lin.tvalue),
                    lin.perigrafi.clone(),
                    lin.sxolio.clone(),
                    lin.parastatiko.clone(),
                    format!("{:.2}", lin.delta),
                ]
            })
            .collect();
        data.reverse();
        data
    }

    /// Flat per-account totals.
    pub fn isozygio_plain(
        &self,
        apo: Option<&str>,
        eos: Option<&str>,
    ) -> BTreeMap<String, IsoTotals> {
        let mut iso: BTreeMap<String, IsoTotals> = BTreeMap::new();
        for trn in self.transactions_filter(apo, eos) {
            for lin in &trn.lines {
                let entry = iso.entry(lin.account_name.clone()).or_default();
                entry.tvalue += lin.value;
                entry.tdebit += lin.debit();
                entry.tcredit += lin.credit();
                entry.tdelta += lin.delta(&self.chart);
            }
        }
        iso
    }

    /// Per-account-prefix totals, aggregating child accounts up the tree.
    pub fn isozygio_tree(
        &self,
        apo: Option<&str>,
        eos: Option<&str>,
    ) -> Result<BTreeMap<String, IsoTotals>, String> {
        let mut fis: BTreeMap<String, IsoTotals> = BTreeMap::new();
        for (acc, vls) in self.isozygio_plain(apo, eos) {
            let objacc = self
                .chart
                .get_account(&acc)
                .ok_or_else(|| format!("Account {} not fount in chart of accounts", acc))?;
            for rac in objacc.tree(".") {
                let isn = fis.entry(rac).or_default();
                isn.tvalue = round2(vls.tvalue + isn.tvalue);
                isn.tdebit = round2(vls.tdebit + isn.tdebit);
                isn.tcredit = round2(vls.tcredit + isn.tcredit);
                isn.tdelta = round2(vls.tdelta + isn.tdelta);
            }
        }
        Ok(fis)
    }

    /// Trial balance rows: [account, formatted balance].
    pub fn model_isozygio(
        &self,
        apo: Option<&str>,
        eos: Option<&str>,
    ) -> Result<Vec<[String; 2]>, String> {
        let tree = self.isozygio_tree(apo, eos)?;
        Ok(tree
            .into_iter()
            .map(|(key, v)| [key, f2gr(v.tvalue)])
            .collect())
    }

    pub fn is_balanced(&self) -> Result<(), String> {
        for trn in self.transactions.values() {
            if !trn.is_balanced() {
                return Err(format!(
                    "Transaction {} {} {} is not balanced",
                    trn.id, trn.date, trn.perigrafi
                ));
            }
        }
        Ok(())
    }

    /// Assigns the next id and stores the transaction. Fails instead of
    /// panicking when the transaction already carries an id.
    pub fn add_transaction(&mut self, transaction: Transaction) -> Result<i32, String> {
        let mut transaction = transaction;
        transaction.set_id(self.last_id + 1)?;
        self.last_id += 1;
        self.transactions.insert(self.last_id, transaction);
        Ok(self.last_id)
    }

    pub fn get_transaction(&self, idv: i32) -> Option<&Transaction> {
        self.transactions.get(&idv)
    }

    pub fn get_transaction_mut(&mut self, idv: i32) -> Option<&mut Transaction> {
        self.transactions.get_mut(&idv)
    }

    pub fn add_validation(&mut self, validation: (String, String, f64)) {
        self.validations.push(validation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser_text::parse_folder;

    fn fixture_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/book01")
    }

    #[test]
    fn test_parse_book01_is_balanced() {
        let (mybook, errors) = parse_folder(fixture_dir().to_str().unwrap()).unwrap();
        assert!(errors.is_empty(), "errors: {:?}", errors);
        assert!(mybook.is_balanced().is_ok());
        assert!(mybook.get_transaction(13).is_some());
    }

    #[test]
    fn test_ypoloipo_and_isozygio() {
        let (book, errors) = parse_folder(fixture_dir().to_str().unwrap()).unwrap();
        assert!(errors.is_empty());
        let iso = book.model_isozygio(None, None).unwrap();
        assert!(!iso.is_empty());
        // every row must name an account and carry a formatted balance
        assert!(iso.iter().all(|row| !row[0].is_empty()));
        assert!(
            iso.iter().all(|row| !row[1].is_empty()),
            "unexpected zero balance (f2gr renders 0 as an empty string): {:?}",
            iso.iter().find(|row| row[1].is_empty())
        );
    }

    #[test]
    fn test_kartella_runs() {
        let (book, _) = parse_folder(fixture_dir().to_str().unwrap()).unwrap();
        let lines = book.kartella("Ταμείο", None, None);
        assert!(!lines.is_empty());
        // last running total equals the full balance
        let last = lines.last().unwrap();
        assert_eq!(last.tvalue, book.ypoloipo("Ταμείο", None));
    }

    #[test]
    fn test_time_series() {
        let (book, _) = parse_folder(fixture_dir().to_str().unwrap()).unwrap();
        let ts = book.time_series("Εξοδα", Grouping::YearMonth, None, None);
        assert!(ts.is_some());
        let ts = ts.unwrap();
        assert!(ts.iter().all(|(k, _, _)| k.len() == 4 || k.len() == 5));
    }
}
