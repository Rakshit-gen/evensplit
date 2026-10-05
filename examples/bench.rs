//! How many payments does settle-up ask for, compared with paying everyone
//! back directly and with the true minimum? And how long does it take?
//!
//!     cargo run --release --example bench
//!
//! Trips are generated from a fixed seed, so the numbers repeat exactly.

use std::collections::BTreeMap;
use std::time::Instant;

use evensplit::settle::settle;
use evensplit::{Expense, Report, Split, Trip};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// A trip like a real one: a few people pay for most things, most costs are
/// shared by everyone, some by a few, some by shares.
fn trip(rng: &mut Rng, people: usize, expenses: usize, round: bool) -> Trip {
    let mut t = Trip::new("bench", "INR");
    t.people = (0..people).map(|i| format!("P{i}")).collect();
    for _ in 0..expenses {
        // Skew towards the first few people paying.
        let payer = (rng.below(people as u64) * rng.below(people as u64) / people as u64) as usize;
        // Round amounts make zero-sum groups more likely, which is where a
        // greedy settle-up can miss the minimum.
        let paise = if round {
            100_00 * (1 + rng.below(20))
        } else {
            50_00 + rng.below(5_000_00)
        };
        let split = match rng.below(20) {
            0..12 => Split::Equal(vec![]),
            12..17 => {
                let mut among: Vec<String> = t
                    .people
                    .iter()
                    .filter(|_| rng.below(2) == 0)
                    .cloned()
                    .collect();
                if among.is_empty() {
                    among.push(t.people[payer].clone());
                }
                Split::Equal(among)
            }
            _ => Split::Shares(
                t.people
                    .iter()
                    .map(|p| (p.clone(), 1 + rng.below(3) as u32))
                    .collect(),
            ),
        };
        t.expenses.push(Expense {
            what: String::new(),
            paid_by: t.people[payer].clone(),
            amount: format!("{}.{:02}", paise / 100, paise % 100),
            currency: None,
            split,
        });
    }
    t
}

/// Payments if everyone pays back each payer directly, netting only within
/// each pair of people.
fn pairwise(t: &Trip) -> usize {
    let mut owed: BTreeMap<(usize, usize), i64> = BTreeMap::new();
    for i in 0..t.expenses.len() {
        let (payer, _, parts) = t.resolve(i).unwrap();
        for (who, part) in parts.into_iter().enumerate() {
            if who != payer && part != 0 {
                let key = (who.min(payer), who.max(payer));
                let sign = if who < payer { 1 } else { -1 };
                *owed.entry(key).or_default() += sign * part;
            }
        }
    }
    owed.values().filter(|&&v| v != 0).count()
}

/// The fewest payments possible: everyone with a balance, minus the most
/// groups they can be split into that each sum to zero (each group of k
/// people needs k - 1 payments). Exponential, fine for up to ~16 people.
fn minimum(nets: &[i64]) -> usize {
    let v: Vec<i64> = nets.iter().copied().filter(|&x| x != 0).collect();
    let n = v.len();
    let mut sum = vec![0i64; 1 << n];
    let mut dp = vec![0usize; 1 << n];
    for mask in 1usize..1 << n {
        let low = mask.trailing_zeros() as usize;
        sum[mask] = sum[mask & (mask - 1)] + v[low];
        let mut best = 0;
        for i in 0..n {
            if mask & (1 << i) != 0 {
                best = best.max(dp[mask ^ (1 << i)]);
            }
        }
        dp[mask] = best + usize::from(sum[mask] == 0);
    }
    n - dp[(1 << n) - 1]
}

fn main() {
    let mut rng = Rng(0x5eed_1234_abcd_0001);
    let trips = 1000;
    for round in [false, true] {
        let kind = if round {
            "whole hundreds of rupees, up to 2,000"
        } else {
            "any amount from 50.00 to 5,050.00"
        };
        println!("{trips} generated trips per row, 30 expenses each, {kind}\n");
        println!("people  pairwise  settle-up  minimum  settle-up = minimum  report time (median)");
        for people in [3, 4, 6, 8, 10, 12] {
            let (mut pw, mut greedy, mut best, mut same) = (0, 0, 0, 0);
            let mut times = Vec::with_capacity(trips);
            for _ in 0..trips {
                let t = trip(&mut rng, people, 30, round);
                let start = Instant::now();
                let report = Report::new(&t).unwrap();
                times.push(start.elapsed());
                let nets: Vec<i64> = report.balances.iter().map(|b| b.net).collect();
                let g = report.payments.len();
                let m = minimum(&nets);
                assert!(g >= m);
                pw += pairwise(&t);
                greedy += g;
                best += m;
                same += usize::from(g == m);
            }
            times.sort();
            let avg = |x: usize| x as f64 / trips as f64;
            println!(
                "{people:>6}  {:>8.2}  {:>9.2}  {:>7.2}  {:>18.1}%  {:>9.1} µs",
                avg(pw),
                avg(greedy),
                avg(best),
                100.0 * same as f64 / trips as f64,
                times[trips / 2].as_secs_f64() * 1e6,
            );
        }

        println!();
    }

    // Settle-up alone on one large group, to show where the O(n^2) loop
    // starts to matter.
    for n in [100, 1000, 5000] {
        let mut nets: Vec<i64> = (0..n - 1)
            .map(|_| rng.below(2_000_001) as i64 - 1_000_000)
            .collect();
        nets.push(-nets.iter().sum::<i64>());
        let start = Instant::now();
        let pays = settle(&nets);
        println!(
            "settle-up for {n} people: {} payments in {:.2} ms",
            pays.len(),
            start.elapsed().as_secs_f64() * 1e3
        );
    }
}
