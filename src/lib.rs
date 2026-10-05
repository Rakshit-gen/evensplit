//! Settle up shared costs: who paid what, who owes whom, and the fewest
//! payments that make everyone even.

pub mod money;
pub mod trip;

pub use trip::{Balance, Expense, Payment, Split, Trip};

/// Everything that can be wrong with a trip, worded for the person who has
/// to fix it.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
    #[error(
        "\"{0}\" isn't an amount in {1}. Write it like 1240.50, with no more decimals than {1} uses."
    )]
    Amount(String, String),
    #[error(
        "\"{0}\" isn't an exchange rate. Write how much one unit is worth in the trip's currency, like 90.25."
    )]
    Rate(String),
    #[error("{0}")]
    Trip(String),
}
