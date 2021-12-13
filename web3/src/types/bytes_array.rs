use serde::{Deserialize, Serialize};

/// A wrapper type for array of bytes.
///
/// Implements `Tokenizable` so can be used to retrieve data from `Solidity` contracts returning `byte8[]`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct BytesArray(pub Vec<u8>);
