//! Amounts as whole minor units (paise, cents, yen). Nothing here touches a
//! float, so 0.1 + 0.2 is always 0.3 and a total always adds back up.

use crate::Error;

/// Digits after the decimal point for a currency code. Most use two; the
/// exceptions are listed so yen are never split into hundredths and dinars
/// keep their third digit.
pub fn decimals(currency: &str) -> u32 {
    match currency {
        "BIF" | "CLP" | "DJF" | "GNF" | "ISK" | "JPY" | "KMF" | "KRW" | "PYG" | "RWF" | "UGX"
        | "VND" | "VUV" | "XAF" | "XOF" | "XPF" => 0,
        "BHD" | "IQD" | "JOD" | "KWD" | "LYD" | "OMR" | "TND" => 3,
        _ => 2,
    }
}

/// Parse "1240.5", "1,240.50" or "-3" into minor units for `currency`.
/// More decimal places than the currency has is an error rather than a
/// silent rounding, because it usually means the wrong currency was picked.
pub fn parse(text: &str, currency: &str) -> Result<i64, Error> {
    let bad = || Error::Amount(text.to_string(), currency.to_string());
    let places = decimals(currency);
    let s: String = text.trim().chars().filter(|&c| c != ',' && c != ' ').collect();
    let (neg, s) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.as_str()),
    };
    let (whole, frac) = s.split_once('.').unwrap_or((s, ""));
    if (whole.is_empty() && frac.is_empty())
        || !whole.chars().all(|c| c.is_ascii_digit())
        || !frac.chars().all(|c| c.is_ascii_digit())
        || frac.len() > places as usize
        || whole.len() > 15
    {
        return Err(bad());
    }
    let mut minor: i64 = if whole.is_empty() { 0 } else { whole.parse().map_err(|_| bad())? };
    for i in 0..places as usize {
        let d = frac.as_bytes().get(i).map_or(0, |b| (b - b'0') as i64);
        minor = minor * 10 + d;
    }
    Ok(if neg { -minor } else { minor })
}

/// Format minor units as "1,240.50", grouping thousands with commas.
pub fn format(minor: i64, currency: &str) -> String {
    let places = decimals(currency);
    let scale = 10i64.pow(places);
    let abs = minor.unsigned_abs();
    let whole = (abs / scale as u64).to_string();
    let mut grouped = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let sign = if minor < 0 { "-" } else { "" };
    if places == 0 {
        format!("{sign}{grouped}")
    } else {
        let frac = abs % scale as u64;
        format!("{sign}{grouped}.{frac:0width$}", width = places as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_ways_of_writing_an_amount() {
        assert_eq!(parse("1240.50", "INR").unwrap(), 124050);
        assert_eq!(parse("1,240.5", "INR").unwrap(), 124050);
        assert_eq!(parse("  12 ", "EUR").unwrap(), 1200);
        assert_eq!(parse(".05", "USD").unwrap(), 5);
        assert_eq!(parse("-3", "USD").unwrap(), -300);
        assert_eq!(parse("1500", "JPY").unwrap(), 1500);
        assert_eq!(parse("1.250", "KWD").unwrap(), 1250);
    }

    #[test]
    fn rejects_amounts_the_currency_cannot_hold() {
        assert!(parse("10.005", "EUR").is_err());
        assert!(parse("15.5", "JPY").is_err());
        assert!(parse("", "EUR").is_err());
        assert!(parse(".", "EUR").is_err());
        assert!(parse("1e5", "EUR").is_err());
        assert!(parse("12.3.4", "EUR").is_err());
        assert!(parse("9999999999999999", "EUR").is_err());
    }

    #[test]
    fn formats_with_grouping_and_fixed_places() {
        assert_eq!(format(124050, "INR"), "1,240.50");
        assert_eq!(format(5, "USD"), "0.05");
        assert_eq!(format(-123456789, "EUR"), "-1,234,567.89");
        assert_eq!(format(100000, "JPY"), "100,000");
        assert_eq!(format(1250, "KWD"), "1.250");
        assert_eq!(format(0, "EUR"), "0.00");
    }

    #[test]
    fn format_and_parse_round_trip() {
        for m in [0, 1, 99, 100, 101, 99999, 1234567, -42] {
            assert_eq!(parse(&format(m, "EUR"), "EUR").unwrap(), m);
        }
    }
}
