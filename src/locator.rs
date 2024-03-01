//! - [Draft Specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md)

use std::collections::HashMap;
use syrup::{Deserialize, Serialize};

/// Onion-specific extensions to the locator module.
#[cfg(feature = "netlayer-onion")]
mod onion;

#[derive(Clone, Deserialize, Serialize)]
#[syrup(name = "ocapn-node",
        deserialize_bound = HintKey: PartialEq + Eq + std::hash::Hash + Deserialize<'__de>; HintValue: Deserialize<'__de>
        )]
pub struct NodeLocator<HintKey, HintValue> {
    pub designator: String,
    #[syrup(as_symbol)]
    pub transport: String,
    #[syrup(with = syrup::optional_map)]
    pub hints: HashMap<HintKey, HintValue>,
}

impl<HKey: std::fmt::Debug, HVal: std::fmt::Debug> std::fmt::Debug for NodeLocator<HKey, HVal> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<ocapn-node {} {} {:?}>",
            self.designator, self.transport, self.hints
        )
    }
}

impl<HKey: std::fmt::Display, HVal: std::fmt::Display> std::fmt::Display
    for NodeLocator<HKey, HVal>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ocapn://{}.{}", self.designator, self.transport)?;
        if !self.hints.is_empty() {
            let mut entries = self.hints.iter();
            let (k, v) = entries.next().unwrap();
            write!(f, "?{k}={v}")?;
            for (k, v) in entries {
                write!(f, "{k}={v}")?;
            }
        }
        Ok(())
    }
}

impl<HintKey, HintValue> PartialEq for NodeLocator<HintKey, HintValue> {
    fn eq(&self, other: &Self) -> bool {
        self.designator == other.designator && self.transport == other.transport
    }
}

impl<HKey, HVal> NodeLocator<HKey, HVal> {
    pub fn new(designator: String, transport: String) -> Self {
        Self {
            designator,
            transport,
            hints: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[syrup(name = "ocapn-sturdyref",
            deserialize_bound = HintKey: PartialEq + Eq + std::hash::Hash + Deserialize<'__de>; HintValue: Deserialize<'__de>

)]
pub struct SturdyRefLocator<HintKey, HintValue> {
    pub node_locator: NodeLocator<HintKey, HintValue>,
    #[syrup(with = syrup::bytes::vec)]
    pub swiss_num: Vec<u8>,
}

impl<HKey, HVal> PartialEq for SturdyRefLocator<HKey, HVal> {
    fn eq(&self, other: &Self) -> bool {
        self.node_locator == other.node_locator && self.swiss_num == other.swiss_num
    }
}

#[cfg(test)]
mod test {
    use super::NodeLocator;
    use crate::locator::SturdyRefLocator;
    use std::collections::HashMap;
    use syrup::Error;

    macro_rules! assert_eq_bstr {
        ($left:expr, $right:expr) => {{
            let lres = $left;
            let rres = $right;
            assert_eq!(
                lres,
                rres,
                "{} != {}",
                unsafe { std::str::from_utf8_unchecked(AsRef::<[u8]>::as_ref(&lres)) },
                unsafe { std::str::from_utf8_unchecked(AsRef::<[u8]>::as_ref(&rres)) }
            );
        }};
    }

    #[test]
    fn serialize_locator() -> Result<(), Error<'static>> {
        assert_eq_bstr!(
            syrup::ser::to_bytes(&NodeLocator::<u8, u8> {
                designator: "testlocator.com".to_owned(),
                transport: "onion".to_owned(),
                hints: HashMap::default()
            })?,
            br#"<10'ocapn-node15"testlocator.com5'onionf>"#
        );
        Ok(())
    }

    #[test]
    fn deserialize_locator() -> Result<(), Error<'static>> {
        assert_eq!(
            syrup::de::from_bytes::<NodeLocator<String, String>>(
                br#"<10'ocapn-node15"testlocator.com5'onionf>"#
            )?,
            NodeLocator {
                designator: "testlocator.com".to_owned(),
                transport: "onion".to_owned(),
                hints: HashMap::default()
            }
        );
        Ok(())
    }

    #[test]
    fn deserialize_sturdyref_locator() -> Result<(), Error<'static>> {
        assert_eq!(
            syrup::de::from_bytes::<SturdyRefLocator<String, String>>(
                br#"<15'ocapn-sturdyref<10'ocapn-node15"testlocator.com5'onionf>3:bef>"#
            )?,
            SturdyRefLocator {
                node_locator: NodeLocator {
                    designator: "testlocator.com".to_owned(),
                    transport: "onion".to_owned(),
                    hints: HashMap::default()
                },
                swiss_num: b"bef".to_vec()
            }
        );
        Ok(())
    }
}
