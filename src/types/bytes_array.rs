use serde::{Deserialize, Serialize};

/// Array of bytes wrapper
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct BytesArray(pub Vec<u8>);
