use rpc;
use {Error};

pub fn to_vector(val: rpc::Value) -> Result<Vec<String>, Error> {
  let invalid = Error::InvalidResponse(format!("Expected vector of strings, got {:?}", val));

  if let rpc::Value::Array(val) = val {
    val.into_iter().map(|v| match v {
     rpc::Value::String(s) => Ok(s),
      _ => Err(invalid.clone()),
    }).collect()
  } else {
    Err(invalid)
  }
}
