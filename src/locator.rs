//! - [Draft Specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md)

use std::collections::HashMap;

pub struct NodeLocator {
    pub designator: String,
    pub transport: String,
    pub hints: HashMap<String, String>,
}

impl NodeLocator {
    /// Serialize this locator to a URI, as described in the
    /// [Locator specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md#uri-serialization).
    pub fn to_url(&self) -> String {
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

pub struct SturdyRefLocator {
    pub node_locator: NodeLocator,
    pub swiss: String,
}

impl SturdyRefLocator {}
