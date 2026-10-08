//! New ids for what the API creates.

/// A seeded id sequence: `prefix_` and 16 hex digits from a splitmix64
/// step, the same generator the app's own ids come from.
#[derive(Debug, Clone)]
pub(crate) struct Ids(u64);

impl Ids {
    pub(crate) const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// An id starting with `prefix` that `taken` does not hold.
    pub(crate) fn fresh(&mut self, prefix: &str, taken: impl Fn(&str) -> bool) -> String {
        loop {
            let id = format!("{prefix}_{:016x}", self.next());
            if !taken(&id) {
                return id;
            }
        }
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}
