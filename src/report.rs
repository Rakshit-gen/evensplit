//! Balances and the settle-up list for a trip, as data and as plain text
//! that reads well when pasted into a group chat.

use serde::Serialize;

use crate::Error;
use crate::money;
use crate::settle::settle;
use crate::trip::{Balance, Trip};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub name: String,
    pub currency: String,
    /// Digits after the point in `currency`, for showing the minor units.
    pub decimals: u32,
    /// All expenses together, in the trip's currency.
    pub total: i64,
    pub balances: Vec<Balance>,
    pub payments: Vec<Owed>,
}

/// A payment still to be made, by name.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Owed {
    pub from: String,
    pub to: String,
    pub amount: i64,
}

impl Report {
    pub fn new(trip: &Trip) -> Result<Report, Error> {
        let balances = trip.balances()?;
        let nets: Vec<i64> = balances.iter().map(|b| b.net).collect();
        let payments = settle(&nets)
            .into_iter()
            .map(|t| Owed {
                from: trip.people[t.from].clone(),
                to: trip.people[t.to].clone(),
                amount: t.amount,
            })
            .collect();
        Ok(Report {
            name: trip.name.clone(),
            currency: trip.currency.clone(),
            decimals: money::decimals(&trip.currency),
            total: balances.iter().map(|b| b.paid).sum(),
            balances,
            payments,
        })
    }

    /// The summary as plain text, one line per person and per payment.
    pub fn text(&self, expenses: usize) -> String {
        let m = |v: i64| money::format(v, &self.currency);
        let mut out = String::new();
        if !self.name.trim().is_empty() {
            out += &format!("{}\n", self.name.trim());
        }
        let noun = if expenses == 1 { "expense" } else { "expenses" };
        out += &format!(
            "{expenses} {noun}, {} {} in all.\n\n",
            m(self.total),
            self.currency
        );
        for b in &self.balances {
            out += &match b.net {
                0 => format!("{} is even\n", b.name),
                n if n > 0 => format!("{} gets back {}\n", b.name, m(n)),
                n => format!("{} owes {}\n", b.name, m(-n)),
            };
        }
        if self.payments.is_empty() {
            out += "\nEveryone is even.\n";
        } else {
            out += "\nTo settle up:\n";
            for p in &self.payments {
                out += &format!("{} pays {} {}\n", p.from, p.to, m(p.amount));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trip::{Expense, Split};

    #[test]
    fn summary_reads_like_a_message() {
        let mut t = Trip::new("Goa, March", "INR");
        t.people = vec!["Asha".into(), "Ben".into(), "Chitra".into()];
        t.expenses.push(Expense {
            what: "Dinner".into(),
            paid_by: "Asha".into(),
            amount: "3721.50".into(),
            currency: None,
            split: Split::Equal(vec![]),
        });
        let r = Report::new(&t).unwrap();
        assert_eq!(
            r.text(t.expenses.len()),
            "Goa, March\n1 expense, 3,721.50 INR in all.\n\n\
             Asha gets back 2,481.00\nBen owes 1,240.50\nChitra owes 1,240.50\n\n\
             To settle up:\nBen pays Asha 1,240.50\nChitra pays Asha 1,240.50\n"
        );
        t.expenses.clear();
        assert!(
            Report::new(&t)
                .unwrap()
                .text(0)
                .ends_with("Everyone is even.\n")
        );
    }
}
