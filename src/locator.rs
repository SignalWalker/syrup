//! - [Draft Specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md)

use syrup::{Deserialize, Serialize};

/// Onion-specific extensions to the locator module.
#[cfg(feature = "netlayer-onion")]
mod onion;

use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
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

impl<HintKey: PartialEq + Eq + std::hash::Hash, HintValue: PartialEq> PartialEq
    for NodeLocator<HintKey, HintValue>
{
    fn eq(&self, other: &Self) -> bool {
        self.designator == other.designator
            && self.transport == other.transport
            && self.hints == other.hints
    }
}

impl<HintKey: PartialEq + Eq + std::hash::Hash, HintValue: PartialEq + Eq> Eq
    for NodeLocator<HintKey, HintValue>
{
}

impl<HKey: std::fmt::Display, HVal: std::fmt::Display> NodeLocator<HKey, HVal> {
    /// Serialize this locator to a URI, as described in the
    /// [Locator specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md#uri-serialization).
    pub fn to_uri(&self) -> String {
        format!(
            "ocapn://{}.{}{}",
            self.designator,
            self.transport,
            if self.hints.is_empty() {
                "".to_owned()
            } else {
                // TODO :: switch to Iterator::intersperse once that's stabilized
                "?".to_owned()
                    + &self
                        .hints
                        .iter()
                        .map(|(key, val)| format!("{key}={val}"))
                        .collect::<Vec<_>>()
                        .join(",")
            }
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[syrup(name = "ocapn-sturdyref",
            deserialize_bound = HintKey: PartialEq + Eq + std::hash::Hash + Deserialize<'__de>; HintValue: Deserialize<'__de>

)]
pub struct SturdyRefLocator<HintKey, HintValue> {
    pub node_locator: NodeLocator<HintKey, HintValue>,
    pub swiss_num: String,
}

impl<HKey: PartialEq + Eq + std::hash::Hash, HVal: PartialEq> PartialEq
    for SturdyRefLocator<HKey, HVal>
{
    fn eq(&self, other: &Self) -> bool {
        self.node_locator == other.node_locator && self.swiss_num == other.swiss_num
    }
}

impl<HKey: PartialEq + Eq + std::hash::Hash, HVal: PartialEq + Eq> Eq
    for SturdyRefLocator<HKey, HVal>
{
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::NodeLocator;
    use crate::locator::SturdyRefLocator;
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
                br#"<15'ocapn-sturdyref<10'ocapn-node15"testlocator.com5'onionf>3"bef>"#
            )?,
            SturdyRefLocator {
                node_locator: NodeLocator {
                    designator: "testlocator.com".to_owned(),
                    transport: "onion".to_owned(),
                    hints: HashMap::default()
                },
                swiss_num: "bef".to_owned()
            }
        );
        Ok(())
    }
}
