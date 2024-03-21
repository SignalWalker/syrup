//! - [Draft Specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md)
use std::{collections::HashMap, num::ParseIntError, str::FromStr, string::FromUtf8Error};

use fluent_uri::{
    component::Scheme,
    encoding::{encoder::Query, EString},
    Uri,
};
use syrup::{Deserialize, Serialize};

#[allow(clippy::doc_markdown)] // false positive on `OCapN`
/// An identifier for a single OCapN node.
///
/// From the [draft specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md):
/// > This identifies an OCapN node, not a specific object. This includes enough information to specify which netlayer and provide that netlayer with all of the information needed to create a bidirectional channel to that node.
#[derive(Clone, Deserialize, Serialize)]
#[syrup(name = "ocapn-node")]
pub struct NodeLocator {
    /// Distinguishes the target node from other nodes accessible through the netlayer specified by
    /// the transport key.
    pub designator: String,
    /// Specifies the netlayer that should be used to access the target node.
    #[syrup(as_symbol)]
    pub transport: String,
    /// Additional connection information.
    #[syrup(with = syrup::optional_map)]
    pub hints: HashMap<syrup::Symbol<String>, String>,
}

impl std::fmt::Debug for NodeLocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&syrup::ser::to_pretty(self).unwrap())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseUriError {
    #[error(transparent)]
    Uri(#[from] fluent_uri::ParseError),
    #[error(transparent)]
    Port(#[from] ParseIntError),
    #[error(transparent)]
    DecodeHint(#[from] FromUtf8Error),
    #[error("expected `ocapn`, found: `{0}`")]
    UnrecognizedScheme(String),
    #[error("no authority component found in parsed uri")]
    MissingAuthority,
    #[error("no transport component found in host str")]
    MissingTransport,
}

impl TryFrom<Uri<&str>> for NodeLocator {
    type Error = ParseUriError;

    fn try_from(uri: Uri<&str>) -> Result<Self, Self::Error> {
        if let Some(scheme) = uri.scheme().map(Scheme::as_str) {
            if !scheme.eq_ignore_ascii_case("ocapn") {
                return Err(ParseUriError::UnrecognizedScheme(scheme.to_owned()));
            }
        }

        let Some(authority) = uri.authority() else {
            return Err(ParseUriError::MissingAuthority);
        };

        #[cfg(feature = "extra-diagnostics")]
        if authority.userinfo().is_some() {
            tracing::warn!("ignoring userinfo in parsed nodelocator uri");
        }

        let (designator, transport) = {
            let host = authority.host();
            let Some((designator, transport)) = host.rsplit_once('.') else {
                return Err(ParseUriError::MissingTransport);
            };
            (designator, transport)
        };

        let mut hints = HashMap::new();

        if let Some(port) = authority.port() {
            hints.insert(syrup::Symbol("port".to_owned()), port.to_owned());
        }

        if let Some(query) = uri.query() {
            for (key, value) in query.split('&').filter_map(|pair| pair.split_once('=')) {
                hints.insert(
                    syrup::Symbol(key.decode().into_string()?.to_string()),
                    value.decode().into_string()?.to_string(),
                );
            }
        }

        Ok(Self {
            designator: designator.to_owned(),
            transport: transport.to_owned(),
            hints,
        })
    }
}

impl FromStr for NodeLocator {
    type Err = ParseUriError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(Uri::parse(s)?)
    }
}

impl std::fmt::Display for NodeLocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO :: percent-encode locator host str
        write!(f, "ocapn://{}.{}", self.designator, self.transport)?;
        if !self.hints.is_empty() {
            let mut query = EString::<Query>::new();
            for (k, v) in self.hints.iter() {
                if !query.is_empty() {
                    query.push_byte(b'&');
                }
                query.encode::<Query>(&k.0);
                query.push_byte(b'=');
                query.encode::<Query>(v);
            }
            write!(f, "?{query}")?;
        }
        Ok(())
    }
}

impl PartialEq for NodeLocator {
    fn eq(&self, other: &Self) -> bool {
        self.designator == other.designator && self.transport == other.transport
    }
}

impl NodeLocator {
    pub fn new(designator: String, transport: String) -> Self {
        Self {
            designator,
            transport,
            hints: HashMap::new(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseSturdyRefUriError {
    #[error(transparent)]
    Locator(#[from] ParseUriError),
    #[error("no path component in parsed uri")]
    MissingPath,
    #[error("uri path component does not start with `s/`")]
    InvalidPath,
}

impl From<fluent_uri::ParseError> for ParseSturdyRefUriError {
    fn from(value: fluent_uri::ParseError) -> Self {
        Self::Locator(ParseUriError::Uri(value))
    }
}

/// A unique identifier for
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[syrup(name = "ocapn-sturdyref")]
pub struct SturdyRefLocator {
    pub node_locator: NodeLocator,
    #[syrup(with = syrup::bytes::vec)]
    pub swiss_num: Vec<u8>,
}

impl std::fmt::Debug for SturdyRefLocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&syrup::ser::to_pretty(self).unwrap())
    }
}

impl TryFrom<Uri<&str>> for SturdyRefLocator {
    type Error = ParseSturdyRefUriError;

    fn try_from(uri: Uri<&str>) -> Result<Self, Self::Error> {
        const SWISS_PREFIX: &[u8] = b"s/";

        let node_locator = NodeLocator::try_from(uri)?;

        let path = uri.path().decode().into_bytes();

        if path.is_empty() {
            return Err(ParseSturdyRefUriError::MissingPath);
        }

        if !path.starts_with(SWISS_PREFIX) {
            return Err(ParseSturdyRefUriError::InvalidPath);
        }

        Ok(Self {
            node_locator,
            swiss_num: path[SWISS_PREFIX.len()..].to_vec(),
        })
    }
}

impl FromStr for SturdyRefLocator {
    type Err = ParseSturdyRefUriError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(Uri::parse(s)?)
    }
}
