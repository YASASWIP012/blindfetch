//! Turning "these rows" into fixed-size batches.
//!
//! PIR hides which row a query asks for, but not how many queries arrive. If
//! a wallet sent one query per memo, the server would learn how many payments
//! it just received. So every round is exactly `batch` queries: real rows
//! first, then dummy rows drawn at random. A wallet with nothing to fetch can
//! still send a full round of dummies, which makes "fetched nothing" look the
//! same as "fetched eight memos".

/// One round of queries. `rows.len()` always equals the plan's batch size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Batch {
    pub rows: Vec<usize>,
    /// How many of `rows` (from the front) are real.
    pub real: usize,
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
                let real: Vec<usize> = rows.iter().skip(i * batch).take(batch).copied().collect();
                let n_real = real.len();
                let mut out = real;
                while out.len() < batch {
                    out.push((random() % num_rows as u64) as usize);
                }
                Batch { rows: out, real: n_real }
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
        assert_eq!(plan.batches[0].rows.len(), 8);
        assert_eq!(plan.batches[0].real, 3);
        assert_eq!(&plan.batches[0].rows[..3], &[5, 9, 40]);
    }

    #[test]
    fn nothing_needed_still_sends_a_round() {
        let plan = QueryPlan::new([], 2048, 8, 1, counter());
        assert_eq!(plan.total_queries(), 8);
        assert_eq!(plan.batches[0].real, 0);
    }

    #[test]
    fn spills_into_more_rounds() {
        let plan = QueryPlan::new(0..19, 2048, 8, 1, counter());
        assert_eq!(plan.batches.len(), 3);
        assert_eq!(plan.batches.iter().map(|b| b.real).collect::<Vec<_>>(), [8, 8, 3]);
        assert!(plan.batches.iter().all(|b| b.rows.len() == 8 && b.rows.iter().all(|&r| r < 2048)));
    }
}
