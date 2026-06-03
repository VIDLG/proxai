use derive_more::{Display, From, Into};
use serde::Serialize;
use valuable::Valuable;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Display,
    From,
    Into,
    Serialize,
    Valuable,
)]
#[display("{_0}")]
pub struct RequestId(u64);

impl RequestId {
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl AsRef<u64> for RequestId {
    fn as_ref(&self) -> &u64 {
        &self.0
    }
}
