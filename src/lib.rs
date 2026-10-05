//! Settle up shared costs: who paid what, who owes whom, and the fewest
//! payments that make everyone even.

pub mod money;

/// Everything that can be wrong with a trip, worded for the person who has
/// to fix it.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("\"{0}\" isn't an amount in {1}. Write it like 1240.50, with no more decimals than {1} uses.")]
    Amount(String, String),
}
