//! Turn balances into a short list of payments that makes everyone even.

use serde::Serialize;

/// One payment: `from` hands `amount` minor units to `to`. People are
/// indexes into the trip's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Transfer {
    pub from: usize,
    pub to: usize,
    pub amount: i64,
}

/// Payments that bring every net balance to zero. `nets` must sum to zero
/// (a trip's balances always do).
///
/// Each step has whoever owes most pay whoever is owed most, as much as
/// settles one of them. Every payment clears at least one person, so there
/// are never more than n - 1 payments; nobody both pays and receives, and no
/// amount goes beyond what someone actually owes. Ties go to whoever comes
/// first in the trip, so the same balances always give the same list.
pub fn settle(nets: &[i64]) -> Vec<Transfer> {
    debug_assert_eq!(nets.iter().sum::<i64>(), 0, "balances must sum to zero");
    let mut left = nets.to_vec();
    let mut out = Vec::new();
    // ponytail: linear scan per payment, O(n^2) overall; a heap if trips
    // ever have thousands of people.
    loop {
        let debtor = pick(&left, |v| -v);
        let creditor = pick(&left, |v| v);
        let (Some(d), Some(c)) = (debtor, creditor) else {
            break;
        };
        let amount = (-left[d]).min(left[c]);
        left[d] += amount;
        left[c] -= amount;
        out.push(Transfer {
            from: d,
            to: c,
            amount,
        });
    }
    out
}

/// The index with the largest positive `key`, first one on ties.
fn pick(values: &[i64], key: impl Fn(i64) -> i64) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, &v) in values.iter().enumerate() {
        if key(v) > 0 && best.is_none_or(|b| key(v) > key(values[b])) {
            best = Some(i);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(from: usize, to: usize, amount: i64) -> Transfer {
        Transfer { from, to, amount }
    }

    /// Apply the payments and check everyone ends at zero.
    fn settles(nets: &[i64], pays: &[Transfer]) -> bool {
        let mut left = nets.to_vec();
        for p in pays {
            assert!(p.amount > 0);
            left[p.from] += p.amount;
            left[p.to] -= p.amount;
        }
        left.iter().all(|&v| v == 0)
    }

    #[test]
    fn nobody_pays_when_everyone_is_even() {
        assert!(settle(&[0, 0, 0]).is_empty());
        assert!(settle(&[]).is_empty());
    }

    #[test]
    fn one_payer_gets_paid_back_by_everyone() {
        let nets = [-100, 300, -100, -100];
        assert_eq!(
            settle(&nets),
            vec![t(0, 1, 100), t(2, 1, 100), t(3, 1, 100)]
        );
    }

    #[test]
    fn largest_debt_meets_largest_credit() {
        let nets = [-500, 200, -100, 400];
        let pays = settle(&nets);
        assert_eq!(pays, vec![t(0, 3, 400), t(0, 1, 100), t(2, 1, 100)]);
        assert!(settles(&nets, &pays));
    }

    #[test]
    fn one_paisa_remainders_still_settle() {
        let nets = [1, -1, 0];
        assert_eq!(settle(&nets), vec![t(1, 0, 1)]);
        let nets = [3334, -1667, -1667];
        assert!(settles(&nets, &settle(&nets)));
        let nets = [1, 1, 1, -3];
        assert_eq!(settle(&nets).len(), 3);
    }

    #[test]
    fn random_balances_settle_in_at_most_n_minus_one_payments() {
        let mut x: u64 = 2463534242;
        let mut next = move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for _ in 0..5000 {
            let n = 2 + (next() % 12) as usize;
            let mut nets: Vec<i64> = (0..n - 1)
                .map(|_| (next() % 20001) as i64 - 10000)
                .collect();
            nets.push(-nets.iter().sum::<i64>());
            let pays = settle(&nets);
            assert!(settles(&nets, &pays));
            let nonzero = nets.iter().filter(|&&v| v != 0).count();
            assert!(pays.len() <= nonzero.saturating_sub(1));
            for p in &pays {
                // Debtors only pay, creditors only receive.
                assert!(nets[p.from] < 0 && nets[p.to] > 0);
            }
        }
    }
}
