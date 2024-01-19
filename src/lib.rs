/// Secure messaging protocol
pub mod captp;
/// Representation of object references
pub mod locator;
/// Secure communication channels between sessions
pub mod netlayer;

pub use syrup;

pub const CAPTP_VERSION: &'static str = "1.0";
