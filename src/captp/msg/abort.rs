use syrup::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[syrup(name = "op:abort")]
pub struct OpAbort {
    pub reason: String,
}

impl std::fmt::Debug for OpAbort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&syrup::ser::to_pretty(self).unwrap())
    }
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
