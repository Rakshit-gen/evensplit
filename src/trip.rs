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
}
