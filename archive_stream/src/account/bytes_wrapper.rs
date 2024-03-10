use serde::{Deserialize, Serialize, Serializer};
use std::borrow::Cow;

#[derive(Deserialize, Debug, Clone)]
pub struct BytesWrapper<'a>(pub Cow<'a, [u8]>);

impl<'a> Serialize for BytesWrapper<'a> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                  where
                    S: Serializer,
  {
    serializer.serialize_bytes(&self.0)
  }
}

impl BytesWrapper<'_> {
  pub fn to_bytes(&self) -> &[u8] {
    &self.0
  }
}
