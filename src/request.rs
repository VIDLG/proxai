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
