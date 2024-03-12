//! - [Draft Specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md)

use std::collections::HashMap;
use syrup::{Deserialize, Serialize};

/// Onion-specific extensions to the locator module.
#[cfg(feature = "netlayer-onion")]
mod onion;

/// An identifier for a single OCapN node.
///
/// From the [draft specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md):
/// > This identifies an OCapN node, not a specific object. This includes enough information to specify which netlayer and provide that netlayer with all of the information needed to create a bidirectional channel to that node.
#[derive(Clone, Deserialize, Serialize)]
#[syrup(name = "ocapn-node",
        deserialize_bound = HintKey: PartialEq + Eq + std::hash::Hash + Deserialize<'__de>; HintValue: Deserialize<'__de>
        )]
pub struct NodeLocator<HintKey, HintValue> {
    /// Distinguishes the target node from other nodes accessible through the netlayer specified by
    /// the transport key.
    pub designator: String,
    /// Specifies the netlayer that should be used to access the target node.
    #[syrup(as_symbol)]
    pub transport: String,
    /// Additional connection information.
    #[syrup(with = syrup::optional_map)]
    pub hints: HashMap<HintKey, HintValue>,
}

impl<HKey, HVal> std::fmt::Debug for NodeLocator<HKey, HVal>
where
    Self: syrup::Serialize,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&syrup::ser::to_pretty(self).unwrap())
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
            // TODO :: escape characters for URI
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

/// A unique identifier for
#[derive(Clone, Deserialize, Serialize)]
#[syrup(name = "ocapn-sturdyref",
    deserialize_bound = HintKey: PartialEq + Eq + std::hash::Hash + Deserialize<'__de>; HintValue: Deserialize<'__de>
)]
pub struct SturdyRefLocator<HintKey, HintValue> {
    pub node_locator: NodeLocator<HintKey, HintValue>,
    #[syrup(with = syrup::bytes::vec)]
    pub swiss_num: Vec<u8>,
}

impl<HKey, HVal> std::fmt::Debug for SturdyRefLocator<HKey, HVal>
where
    Self: syrup::Serialize,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&syrup::ser::to_pretty(self).unwrap())
    }
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
            #[allow(unsafe_code)]
            {
                assert_eq!(
                    lres,
                    rres,
                    "{} != {}",
                    unsafe { std::str::from_utf8_unchecked(AsRef::<[u8]>::as_ref(&lres)) },
                    unsafe { std::str::from_utf8_unchecked(AsRef::<[u8]>::as_ref(&rres)) }
                );
            }
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
