//! A trip as it is saved: people, expenses and the payments already made.
//! Amounts are kept as the text people typed ("1240.50") so the file stays
//! readable and editable by hand; they are parsed into minor units on use.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::Error;
use crate::money::{self, Rate};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trip {
    pub name: String,
    /// The currency balances and payments are in, like "INR".
    pub currency: String,
    /// What one unit of each other currency is worth in `currency`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub rates: BTreeMap<String, String>,
    pub people: Vec<String>,
    #[serde(default)]
    pub expenses: Vec<Expense>,
    /// Money handed over to settle up, always in the trip's currency.
    #[serde(default)]
    pub payments: Vec<Payment>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Expense {
    pub what: String,
    pub paid_by: String,
    pub amount: String,
    /// Left out when it is the trip's currency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(default)]
    pub split: Split,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Split {
    /// Equal parts among these people; an empty list means everyone.
    Equal(Vec<String>),
    /// Parts in proportion, like two shares for a couple and one for a single.
    Shares(BTreeMap<String, u32>),
    /// Exact amounts in the expense's currency, adding up to its total.
    Exact(BTreeMap<String, String>),
}

impl Default for Split {
    fn default() -> Self {
        Split::Equal(Vec::new())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Payment {
    pub from: String,
    pub to: String,
    pub amount: String,
}

fn invalid(msg: String) -> Error {
    Error::Trip(msg)
}

impl Trip {
    pub fn new(name: &str, currency: &str) -> Trip {
        Trip {
            name: name.to_string(),
            currency: currency.to_string(),
            rates: BTreeMap::new(),
            people: Vec::new(),
            expenses: Vec::new(),
            payments: Vec::new(),
        }
    }

    fn person(&self, name: &str, context: &str) -> Result<usize, Error> {
        self.people.iter().position(|p| p == name).ok_or_else(|| {
            invalid(format!(
                "{context} mentions {name}, who isn't on the trip. Add them, or fix the name."
            ))
        })
    }

    fn rate(&self, currency: &str) -> Result<Rate, Error> {
        match self.rates.get(currency) {
            Some(r) => Rate::parse(r),
            None => Err(invalid(format!(
                "There's no exchange rate for {currency}. Add how much 1 {currency} is in {}.",
                self.currency
            ))),
        }
    }

    /// Who paid expense `index`, what it cost in the trip's currency, and
    /// each person's part of that cost, in the order of `people`. The parts
    /// always add up to the cost exactly.
    pub fn resolve(&self, index: usize) -> Result<(usize, i64, Vec<i64>), Error> {
        let e = &self.expenses[index];
        let label = if e.what.trim().is_empty() {
            format!("Expense {}", index + 1)
        } else {
            format!("\"{}\"", e.what.trim())
        };
        let payer = self.person(&e.paid_by, &label)?;
        let cur = e.currency.as_deref().unwrap_or(&self.currency);
        let amount = money::parse(&e.amount, cur)?;
        if amount <= 0 {
            return Err(invalid(format!(
                "{label} has an amount of {}. Expenses need an amount above zero.",
                e.amount
            )));
        }
        let cost = if cur == self.currency {
            amount
        } else {
            self.rate(cur)?.convert(amount, cur, &self.currency)
        };
        let n = self.people.len();
        let mut weights = vec![0u64; n];
        match &e.split {
            Split::Equal(among) if among.is_empty() => weights.fill(1),
            Split::Equal(among) => {
                for name in among {
                    weights[self.person(name, &label)?] = 1;
                }
            }
            Split::Shares(shares) => {
                for (name, &w) in shares {
                    weights[self.person(name, &label)?] = w as u64;
                }
            }
            Split::Exact(parts) => {
                let mut sum = 0;
                for (name, text) in parts {
                    let part = money::parse(text, cur)?;
                    if part < 0 {
                        return Err(invalid(format!(
                            "{label} gives {name} a negative part. Use zero or more."
                        )));
                    }
                    weights[self.person(name, &label)?] = part as u64;
                    sum += part;
                }
                if sum != amount {
                    return Err(invalid(format!(
                        "{label} is {} {cur} but the exact parts add up to {} {cur}. Make them match.",
                        money::format(amount, cur),
                        money::format(sum, cur)
                    )));
                }
            }
        }
        if weights.iter().all(|&w| w == 0) {
            return Err(invalid(format!(
                "{label} isn't split between anyone. Pick at least one person."
            )));
        }
        // Exact parts in another currency are used as weights, so the
        // converted cost is shared in the same proportions and still adds
        // up. In the trip's own currency they come out unchanged.
        Ok((payer, cost, money::allocate(cost, &weights, index)))
    }

    /// Check the parts that don't depend on any one expense.
    fn check_header(&self) -> Result<(), Error> {
        if !is_code(&self.currency) {
            return Err(invalid(format!(
                "\"{}\" isn't a currency code. Use three capital letters, like INR or EUR.",
                self.currency
            )));
        }
        for code in self.rates.keys() {
            if !is_code(code) {
                return Err(invalid(format!(
                    "\"{code}\" isn't a currency code. Use three capital letters, like INR or EUR."
                )));
            }
        }
        for (i, p) in self.people.iter().enumerate() {
            if p.trim().is_empty() || p.trim() != p {
                return Err(invalid(format!(
                    "\"{p}\" can't be used as a name. Names need at least one letter and no spaces at either end."
                )));
            }
            if self.people[..i].contains(p) {
                return Err(invalid(format!(
                    "{p} is on the trip twice. Give each person a different name."
                )));
            }
        }
        Ok(())
    }
}

fn is_code(s: &str) -> bool {
    s.len() == 3 && s.bytes().all(|b| b.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_hand_written_trip() {
        let t: Trip = serde_json::from_str(
            r#"{
                "name": "Goa",
                "currency": "INR",
                "rates": { "EUR": "90.25" },
                "people": ["Asha", "Ben"],
                "expenses": [
                    { "what": "Dinner", "paid_by": "Asha", "amount": "1240.50" },
                    { "what": "Boat", "paid_by": "Ben", "amount": "40", "currency": "EUR",
                      "split": { "shares": { "Asha": 2, "Ben": 1 } } }
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(t.expenses[0].split, Split::Equal(vec![]));
        assert!(t.payments.is_empty());
        assert!(t.check_header().is_ok());
        let back: Trip = serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn header_problems_are_named() {
        let mut t = Trip::new("x", "inr");
        assert!(t.check_header().unwrap_err().to_string().contains("\"inr\""));
        t.currency = "INR".into();
        t.people = vec!["Asha".into(), "Asha".into()];
        assert!(t.check_header().unwrap_err().to_string().contains("twice"));
        t.people = vec![" Asha".into()];
        assert!(t.check_header().is_err());
    }

    fn trip() -> Trip {
        let mut t = Trip::new("Goa", "INR");
        t.people = vec!["Asha".into(), "Ben".into(), "Chitra".into()];
        t.rates.insert("EUR".into(), "90.25".into());
        t
    }

    fn add(t: &mut Trip, paid_by: &str, amount: &str, currency: Option<&str>, split: Split) {
        t.expenses.push(Expense {
            what: "x".into(),
            paid_by: paid_by.into(),
            amount: amount.into(),
            currency: currency.map(Into::into),
            split,
        });
    }

    #[test]
    fn equal_split_rotates_the_odd_paisa() {
        let mut t = trip();
        for _ in 0..3 {
            add(&mut t, "Asha", "100", None, Split::Equal(vec![]));
        }
        assert_eq!(t.resolve(0).unwrap(), (0, 10000, vec![3334, 3333, 3333]));
        assert_eq!(t.resolve(1).unwrap().2, vec![3333, 3334, 3333]);
        assert_eq!(t.resolve(2).unwrap().2, vec![3333, 3333, 3334]);
    }

    #[test]
    fn equal_split_among_some_people() {
        let mut t = trip();
        add(&mut t, "Chitra", "0.01", None, Split::Equal(vec!["Ben".into(), "Chitra".into()]));
        let (payer, cost, parts) = t.resolve(0).unwrap();
        assert_eq!((payer, cost), (2, 1));
        assert_eq!(parts.iter().sum::<i64>(), 1);
        assert_eq!(parts[0], 0);
    }

    #[test]
    fn shares_split_in_proportion() {
        let mut t = trip();
        let shares = [("Asha".to_string(), 2), ("Ben".to_string(), 1)].into();
        add(&mut t, "Ben", "300", None, Split::Shares(shares));
        assert_eq!(t.resolve(0).unwrap().2, vec![20000, 10000, 0]);
    }

    #[test]
    fn exact_split_must_add_up() {
        let mut t = trip();
        let parts = [("Asha".to_string(), "70".to_string()), ("Ben".to_string(), "30.50".to_string())];
        add(&mut t, "Asha", "100.50", None, Split::Exact(parts.clone().into()));
        assert_eq!(t.resolve(0).unwrap().2, vec![7000, 3050, 0]);
        t.expenses[0].amount = "100".into();
        let err = t.resolve(0).unwrap_err().to_string();
        assert!(err.contains("add up to 100.50 INR"), "{err}");
    }

    #[test]
    fn foreign_expenses_convert_once_and_split_exactly() {
        let mut t = trip();
        // 33.33 EUR at 90.25 is 3008.0325 INR, which rounds to 3008.03.
        add(&mut t, "Ben", "33.33", Some("EUR"), Split::Equal(vec![]));
        let (_, cost, parts) = t.resolve(0).unwrap();
        assert_eq!(cost, 300803);
        assert_eq!(parts, vec![100268, 100268, 100267]);
        let parts = [("Asha".to_string(), "10".to_string()), ("Chitra".to_string(), "23.33".to_string())];
        t.expenses[0].split = Split::Exact(parts.into());
        let (_, cost, parts) = t.resolve(0).unwrap();
        assert_eq!(parts.iter().sum::<i64>(), cost);
        assert_eq!(parts[0], 90250);
    }

    #[test]
    fn expense_problems_say_what_to_fix() {
        let mut t = trip();
        add(&mut t, "Dev", "10", None, Split::Equal(vec![]));
        assert!(t.resolve(0).unwrap_err().to_string().contains("Dev, who isn't on the trip"));
        t.expenses[0].paid_by = "Asha".into();
        t.expenses[0].currency = Some("USD".into());
        assert!(t.resolve(0).unwrap_err().to_string().contains("no exchange rate for USD"));
        t.expenses[0].currency = None;
        t.expenses[0].amount = "0".into();
        assert!(t.resolve(0).unwrap_err().to_string().contains("above zero"));
        t.expenses[0].amount = "10".into();
        t.expenses[0].split = Split::Shares([("Ben".to_string(), 0)].into());
        assert!(t.resolve(0).unwrap_err().to_string().contains("isn't split"));
    }
}
