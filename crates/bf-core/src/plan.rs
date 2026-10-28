//! Turning "these rows" into fixed-size batches.
//!
//! PIR hides which row a query asks for, but not how many queries arrive. If
//! a wallet sent one query per memo, the server would learn how many payments
//! it just received. So every round is exactly `batch` queries: the real rows
//! plus dummy rows drawn at random. A wallet with nothing to fetch can
//! still send a full round of dummies, which makes "fetched nothing" look the
//! same as "fetched four memos".
//!
//! Real queries are shuffled in among the dummies, and clients decode and
//! check every answer, so no slot position and no failure tells the server
//! which queries mattered.

/// One round of queries. `rows.len()` always equals the plan's batch size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Batch {
    pub rows: Vec<usize>,
    /// Which slots the client needs. Only the client knows this.
    pub real: Vec<bool>,
}

impl Batch {
    pub fn real_count(&self) -> usize {
        self.real.iter().filter(|r| **r).count()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryPlan {
    pub batches: Vec<Batch>,
}

impl QueryPlan {
    /// Plans rounds for `needed` rows in a database of `num_rows` rows.
    ///
    /// Duplicates are dropped. At least `min_rounds` batches are produced even
    /// when nothing is needed. `random` must return uniformly random values;
    /// it picks the dummy rows.
    pub fn new(
        needed: impl IntoIterator<Item = usize>,
        num_rows: usize,
        batch: usize,
        min_rounds: usize,
        mut random: impl FnMut() -> u64,
    ) -> Self {
        assert!(batch > 0 && num_rows > 0);
        let mut rows: Vec<usize> = needed.into_iter().collect();
        rows.sort_unstable();
        rows.dedup();
        assert!(rows.iter().all(|&r| r < num_rows), "row out of range");

        let rounds = rows.len().div_ceil(batch).max(min_rounds);
        let batches = (0..rounds)
            .map(|i| {
                let mut slots: Vec<(usize, bool)> =
                    rows.iter().skip(i * batch).take(batch).map(|&r| (r, true)).collect();
                while slots.len() < batch {
                    slots.push(((random() % num_rows as u64) as usize, false));
                }
                // Fisher-Yates, so real queries sit in random slots.
                for j in (1..slots.len()).rev() {
                    let k = (random() % (j as u64 + 1)) as usize;
                    slots.swap(j, k);
                }
                Batch { rows: slots.iter().map(|s| s.0).collect(), real: slots.iter().map(|s| s.1).collect() }
            })
            .collect();
        QueryPlan { batches }
    }

    pub fn total_queries(&self) -> usize {
        self.batches.iter().map(|b| b.rows.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counter() -> impl FnMut() -> u64 {
        let mut x = 0x9e37_79b9_7f4a_7c15u64;
        move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        }
    }

    #[test]
    fn every_round_is_full() {
        let plan = QueryPlan::new([5, 9, 9, 40], 2048, 8, 1, counter());
        assert_eq!(plan.batches.len(), 1);
        let b = &plan.batches[0];
        assert_eq!(b.rows.len(), 8);
        assert_eq!(b.real_count(), 3);
        let mut real: Vec<usize> = b.rows.iter().zip(&b.real).filter(|(_, r)| **r).map(|(x, _)| *x).collect();
        real.sort();
        assert_eq!(real, [5, 9, 40]);
    }

    #[test]
    fn nothing_needed_still_sends_a_round() {
        let plan = QueryPlan::new([], 2048, 8, 1, counter());
        assert_eq!(plan.total_queries(), 8);
        assert_eq!(plan.batches[0].real_count(), 0);
    }

    #[test]
    fn real_slots_move() {
        let mut rng = counter();
        let firsts: std::collections::HashSet<usize> = (0..50)
            .map(|_| QueryPlan::new([7], 2048, 4, 1, &mut rng).batches[0].real.iter().position(|r| *r).unwrap())
            .collect();
        assert!(firsts.len() > 1, "the real query always sat in the same slot");
    }

    #[test]
    fn spills_into_more_rounds() {
        let plan = QueryPlan::new(0..19, 2048, 8, 1, counter());
        assert_eq!(plan.batches.len(), 3);
        assert_eq!(plan.batches.iter().map(|b| b.real_count()).collect::<Vec<_>>(), [8, 8, 3]);
        assert!(plan.batches.iter().all(|b| b.rows.len() == 8 && b.rows.iter().all(|&r| r < 2048)));
    }
}
