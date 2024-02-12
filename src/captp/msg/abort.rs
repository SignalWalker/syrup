use syrup::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[syrup(name = "op:abort")]
pub struct OpAbort {
    pub reason: String,
}

impl From<String> for OpAbort {
    #[inline]
    fn from(reason: String) -> Self {
        Self { reason }
    }
}

impl From<&str> for OpAbort {
    #[inline]
    fn from(value: &str) -> Self {
        Self {
            reason: value.into(),
        }
    }
}
