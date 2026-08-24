//! Differential test: the Rust port must produce exactly the same results
//! as the original Python implementation (golden file generated with the
//! PySide6-era `qlogistiki` code on the `book01` fixture).

use std::path::Path;

use qhomeacc_core::date_groups::Grouping;
use qhomeacc_core::parser_text::parse_folder;
use qhomeacc_core::utils::{f2gr, gr2float, round2};

fn fixture_dir() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/book01")
        .to_str()
        .unwrap()
        .to_string()
}

#[test]
fn test_matches_python_golden() {
    let golden_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/py_golden.json");
    let golden: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(golden_path).unwrap()).unwrap();

    let (book, errors) = parse_folder(&fixture_dir()).unwrap();
    assert!(errors.is_empty());

    // isozygio rows must match exactly
    let iso = book.model_isozygio(None, None).unwrap();
    let golden_iso: Vec<Vec<String>> = serde_json::from_value(golden["isozygio"].clone()).unwrap();
    let rust_iso: Vec<Vec<String>> = iso.iter().map(|row| row.to_vec()).collect();
    assert_eq!(rust_iso, golden_iso, "isozygio mismatch");

    // kartella of "Ταμείο": id, date, running value, delta
    let kart = book.kartella("Ταμείο", None, None);
    let golden_kart: Vec<Vec<serde_json::Value>> =
        serde_json::from_value(golden["kartella_tameio"].clone()).unwrap();
    assert_eq!(kart.len(), golden_kart.len(), "kartella length");
    for (l, g) in kart.iter().zip(golden_kart.iter()) {
        assert_eq!(l.id.to_string(), g[0].as_str().unwrap());
        assert_eq!(
            l.date.format("%Y-%m-%d").to_string(),
            g[1].as_str().unwrap()
        );
        let gtvalue: f64 = g[2].as_str().unwrap().parse().unwrap();
        let gtdelta: f64 = g[3].as_str().unwrap().parse().unwrap();
        assert_eq!(round2(l.tvalue), gtvalue, "tvalue id {}", l.id);
        assert_eq!(round2(l.tdelta), gtdelta, "tdelta id {}", l.id);
    }

    // time series of "Εξοδα" grouped by month
    let ts = book
        .time_series("Εξοδα", Grouping::YearMonth, None, None)
        .unwrap();
    let golden_ts: Vec<Vec<serde_json::Value>> =
        serde_json::from_value(golden["ts_exoda_month"].clone()).unwrap();
    assert_eq!(ts.len(), golden_ts.len(), "time series length");
    for ((k, t, v), g) in ts.iter().zip(golden_ts.iter()) {
        assert_eq!(k, g[0].as_str().unwrap());
        assert_eq!(round2(*t), g[1].as_f64().unwrap(), "total at {}", k);
        assert_eq!(round2(*v), g[2].as_f64().unwrap(), "value at {}", k);
    }

    // ypoloipo of "Πιστωτές"
    let ypol = book.ypoloipo("Πιστωτές", None);
    assert_eq!(ypol, golden["ypoloipo_54"].as_f64().unwrap());

    // Every formatted balance must read back as the number it came from.
    let tree = book.isozygio_tree(None, None).unwrap();
    for row in &iso {
        let tvalue = tree[&row[0]].tvalue;
        assert_eq!(row[1], f2gr(tvalue), "formatting of {}", row[0]);
        let parsed = if row[1].is_empty() {
            0.0
        } else {
            gr2float(&row[1]).unwrap_or_else(|| panic!("{} is not a Greek number", row[1]))
        };
        assert_eq!(parsed, round2(tvalue), "round-trip of {}", row[0]);
    }
}
