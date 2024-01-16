use arti_client::IntoTorAddr;

use super::NodeLocator;

impl<HKey, HVal> IntoTorAddr for &NodeLocator<HKey, HVal> {
    fn into_tor_addr(self) -> Result<arti_client::TorAddr, arti_client::TorAddrError> {
        self.designator.as_str().into_tor_addr()
    }
}
