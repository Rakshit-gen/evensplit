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
    let s: String = text
        .trim()
        .chars()
        .filter(|&c| c != ',' && c != ' ')
        .collect();
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
    let mut minor: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().map_err(|_| bad())?
    };
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
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
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

/// An exchange rate written as a decimal, "90.25" meaning one unit of the
/// foreign currency is worth 90.25 of the trip's base currency. Kept as
/// digits and a scale so converting never goes through a float.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rate {
    digits: i128,
    scale: u32,
}

impl Rate {
    pub fn parse(text: &str) -> Result<Rate, Error> {
        let bad = || Error::Rate(text.to_string());
        let t = text.trim();
        let (whole, frac) = t.split_once('.').unwrap_or((t, ""));
        if (whole.is_empty() && frac.is_empty())
            || !whole
                .chars()
                .chain(frac.chars())
                .all(|c| c.is_ascii_digit())
            || whole.len() + frac.len() > 24
        {
            return Err(bad());
        }
        let digits: i128 = format!("{whole}{frac}").parse().map_err(|_| bad())?;
        if digits == 0 {
            return Err(bad());
        }
        Ok(Rate {
            digits,
            scale: frac.len() as u32,
        })
    }

    /// Convert `minor` units of `from` into minor units of `to`, rounding
    /// half away from zero. This is the only place a trip rounds money
    /// between currencies, and it happens once per expense, before any
    /// splitting, so the split itself stays exact.
    ///
    /// None when the result doesn't fit. A 24-digit rate times a 15-digit
    /// amount is past even i128, and the old cast to i64 wrapped round to a
    /// wrong amount without a word.
    pub fn convert(&self, minor: i64, from: &str, to: &str) -> Option<i64> {
        let num = (minor as i128)
            .checked_mul(self.digits)?
            .checked_mul(10i128.pow(decimals(to)))?;
        let den = 10i128.pow(self.scale + decimals(from));
        let q = num / den;
        let r = num % den;
        let out = if 2 * r.abs() >= den {
            q + num.signum()
        } else {
            q
        };
        i64::try_from(out).ok()
    }
}

/// Split `total` in proportion to `weights` so the parts add up to exactly
/// `total`. Everyone first gets the rounded-down share; the units left over
/// go one each to the largest fractions. Ties are broken starting from
/// position `start` and wrapping round, so callers can rotate who picks up
/// the odd paisa instead of it always landing on the same person.
pub fn allocate(total: i64, weights: &[u64], start: usize) -> Vec<i64> {
    let sum: u128 = weights.iter().map(|&w| w as u128).sum();
    if sum == 0 {
        return vec![0; weights.len()];
    }
    let mag = total.unsigned_abs() as u128;
    let mut parts: Vec<i64> = Vec::with_capacity(weights.len());
    let mut rems: Vec<(u128, usize)> = Vec::with_capacity(weights.len());
    for (i, &w) in weights.iter().enumerate() {
        let exact = mag * w as u128;
        parts.push((exact / sum) as i64);
        rems.push((exact % sum, i));
    }
    let n = weights.len();
    let left = mag as i64 - parts.iter().sum::<i64>();
    // Largest remainder first; among equals, nearest to `start` first.
    rems.sort_by_key(|&(r, i)| (std::cmp::Reverse(r), (i + n - start % n) % n));
    for &(_, i) in rems.iter().take(left as usize) {
        parts[i] += 1;
    }
    if total < 0 {
        parts.iter_mut().for_each(|p| *p = -*p);
    }
    parts
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
    fn conversion_that_does_not_fit_is_none() {
        let huge = Rate::parse("999999999999999999999999").unwrap();
        assert_eq!(huge.convert(100_000_000_000_000_000, "EUR", "INR"), None);
        let big = Rate::parse("1000").unwrap();
        assert_eq!(big.convert(i64::MAX / 10, "EUR", "INR"), None);
    }

    #[test]
    fn format_and_parse_round_trip() {
        for m in [0, 1, 99, 100, 101, 99999, 1234567, -42] {
            assert_eq!(parse(&format(m, "EUR"), "EUR").unwrap(), m);
        }
    }

    #[test]
    fn converts_between_currencies_and_rounds_half_away_from_zero() {
        let eur = Rate::parse("90.25").unwrap();
        assert_eq!(eur.convert(1000, "EUR", "INR").unwrap(), 90250);
        // 0.01 EUR is 0.9025 INR, which rounds to 0.90.
        assert_eq!(eur.convert(1, "EUR", "INR").unwrap(), 90);
        // 0.02 EUR is 1.805 INR, a half, which goes up.
        assert_eq!(eur.convert(2, "EUR", "INR").unwrap(), 181);
        assert_eq!(eur.convert(-2, "EUR", "INR").unwrap(), -181);
        let yen = Rate::parse("0.0061").unwrap();
        assert_eq!(yen.convert(1500, "JPY", "EUR").unwrap(), 915);
        let usd = Rate::parse("0.3765").unwrap();
        assert_eq!(usd.convert(100, "USD", "KWD").unwrap(), 377);
        assert_eq!(
            Rate::parse("1")
                .unwrap()
                .convert(12345, "EUR", "USD")
                .unwrap(),
            12345
        );
    }

    #[test]
    fn rejects_rates_that_are_not_positive_numbers() {
        for bad in ["", "0", "0.000", "-1", "abc", "1,5", "."] {
            assert!(Rate::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn allocation_hands_out_the_leftover_and_never_loses_a_unit() {
        assert_eq!(allocate(100, &[1, 1, 1], 0), vec![34, 33, 33]);
        assert_eq!(allocate(100, &[1, 1, 1], 1), vec![33, 34, 33]);
        assert_eq!(allocate(100, &[1, 1, 1], 5), vec![33, 33, 34]);
        assert_eq!(allocate(-100, &[1, 1, 1], 0), vec![-34, -33, -33]);
        assert_eq!(allocate(1000, &[2, 1, 1], 0), vec![500, 250, 250]);
        // The biggest fraction wins the leftover, whatever the rotation.
        assert_eq!(allocate(10, &[1, 2], 0), vec![3, 7]);
        assert_eq!(allocate(1, &[1, 1, 1, 1], 2), vec![0, 0, 1, 0]);
        assert_eq!(allocate(0, &[1, 1], 0), vec![0, 0]);
        assert_eq!(allocate(50, &[0, 0], 0), vec![0, 0]);
    }

    #[test]
    fn allocation_always_adds_back_up() {
        // A small deterministic generator so the test needs no crates.
        let mut x: u64 = 0x9e3779b97f4a7c15;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for _ in 0..20_000 {
            let n = 1 + (next() % 9) as usize;
            let weights: Vec<u64> = (0..n).map(|_| 1 + next() % 7).collect();
            let total = (next() % 2_000_000) as i64 - 1_000_000;
            let start = (next() % 10) as usize;
            let parts = allocate(total, &weights, start);
            assert_eq!(parts.iter().sum::<i64>(), total);
            let sum: u64 = weights.iter().sum();
            for (p, w) in parts.iter().zip(&weights) {
                // Nobody is off their fair share by a whole unit or more.
                let fair = total as f64 * *w as f64 / sum as f64;
                assert!((*p as f64 - fair).abs() < 1.0, "{p} vs {fair}");
            }
        }
    }
}
