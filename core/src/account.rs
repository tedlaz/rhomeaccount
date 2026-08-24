//! Port of `qlogistiki/account.py`: chart of accounts and account categories.

use std::collections::BTreeMap;

use crate::utils::account_tree;

pub const OMADES_TYPES_GR: &[(&str, &str)] = &[
    ("1", "pagia"),
    ("2", "apothemata"),
    ("3", "apaitiseis"),
    ("4", "kefalaio"),
    ("5", "ypoxreoseis"),
    ("54.00", "fpa"),
    ("6", "ejoda"),
    ("7", "esoda"),
    ("8", "anorgana"),
];

pub const KATS: &[&str] = &[
    "pagia",
    "apothemata",
    "apaitiseis",
    "kefalaio",
    "ypoxreoseis",
    "esoda",
    "ejoda",
    "anorgana",
    "fpa",
];

/// `LogistikoSxedio` — a named chart of accounts.
#[derive(Debug, Clone)]
pub struct ChartOfAccounts {
    pub name: String,
    /// Ordered list of (account-prefix, category) pairs.
    pub categories: Vec<(String, String)>,
    pub accounts: BTreeMap<String, Account>,
}

impl ChartOfAccounts {
    pub fn new(name: &str, types: Vec<(String, String)>) -> Self {
        assert!(types.iter().all(|(_, t)| KATS.contains(&t.as_str())));
        ChartOfAccounts {
            name: name.to_string(),
            categories: types,
            accounts: BTreeMap::new(),
        }
    }

    /// Adds an account if its name starts with one of the category prefixes.
    pub fn add_account(&mut self, account_name: &str) -> Result<(), String> {
        let starts_with_any = self
            .categories
            .iter()
            .any(|(prefix, _)| account_name.starts_with(prefix.as_str()));
        if !starts_with_any {
            return Err(format!("Error account name: {}", account_name));
        }
        self.accounts
            .entry(account_name.to_string())
            .or_insert_with(|| Account {
                name: account_name.to_string(),
            });
        Ok(())
    }

    /// `LogistikoSxedio.full` — build chart from name, types and account list.
    pub fn full(
        bname: &str,
        acc_types: Vec<(String, String)>,
        accountlist: &[String],
    ) -> Result<Self, String> {
        let mut new_ls = ChartOfAccounts::new(bname, acc_types);
        new_ls.add_accounts_from_list(accountlist)?;
        Ok(new_ls)
    }

    pub fn add_accounts_from_list(&mut self, acclist: &[String]) -> Result<(), String> {
        for account in acclist {
            self.add_account(account)?;
        }
        Ok(())
    }

    pub fn get_account(&self, account_name: &str) -> Option<&Account> {
        self.accounts.get(account_name)
    }

    pub fn is_valid_account(&self, account_name: &str) -> bool {
        self.accounts.contains_key(account_name)
    }

    /// All categories whose prefix the account name starts with.
    /// For "54.00.00.013" with default types this is ["ypoxreoseis", "fpa"].
    pub fn account_type(&self, account_name: &str) -> Vec<String> {
        let mut typs = Vec::new();
        for (acc_start, typ) in &self.categories {
            if account_name.starts_with(acc_start.as_str()) {
                typs.push(typ.clone());
            }
        }
        typs
    }

    pub fn is_fpa(&self, account_name: &str) -> bool {
        self.account_type(account_name).iter().any(|t| t == "fpa")
    }

    /// True when the account belongs to esoda/ypoxreoseis/kefalaio,
    /// meaning values are sign-reversed for display (`delta`).
    pub fn is_reverse(&self, account_name: &str) -> bool {
        self.account_type(account_name)
            .iter()
            .any(|t| t == "esoda" || t == "ypoxreoseis" || t == "kefalaio")
    }
}

/// A single account (only its name; the chart carries the semantics).
#[derive(Debug, Clone)]
pub struct Account {
    pub name: String,
}

impl Account {
    pub fn tree(&self, splitter: &str) -> Vec<String> {
        account_tree(&self.name, false, splitter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types() -> Vec<(String, String)> {
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
        .collect()
    }

    #[test]
    fn test_ls1() {
        let ls1 = ChartOfAccounts::new("gr", types());
        assert_eq!(
            ls1.account_type("54.00.00.013"),
            vec!["ypoxreoseis".to_string(), "fpa".to_string()]
        );
    }

    #[test]
    fn test_acc_tree() {
        let mut ls1 = ChartOfAccounts::new("gr", types());
        ls1.add_account("20.00.00").unwrap();
        let acc = Account {
            name: "Aa.Bb.Cc".to_string(),
        };
        assert_eq!(
            acc.tree("."),
            vec![
                "Aa".to_string(),
                "Aa.Bb".to_string(),
                "Aa.Bb.Cc".to_string()
            ]
        );
        let mut rev = acc.tree(".");
        rev.reverse();
        assert_eq!(
            rev,
            vec![
                "Aa.Bb.Cc".to_string(),
                "Aa.Bb".to_string(),
                "Aa".to_string()
            ]
        );
    }

    #[test]
    fn test_acc_reverse() {
        let ls1 = ChartOfAccounts::new("gr", types());
        assert!(ls1.is_reverse("70.01.013"));
        assert!(!ls1.is_reverse("20.01.013"));
        assert!(ls1.is_fpa("54.00.10.001"));
    }

    #[test]
    fn test_invalid_account() {
        let mut ls1 = ChartOfAccounts::new("gr", types());
        assert!(ls1.add_account("90.00.00").is_err());
        assert!(ls1.add_account("80.00.00").is_ok());
    }
}
