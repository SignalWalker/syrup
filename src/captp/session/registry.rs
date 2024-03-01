use crate::captp::object::DeliverySender;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct SwissRegistry {
    map: dashmap::DashMap<Vec<u8>, DeliverySender>,
}

impl SwissRegistry {
    pub fn new() -> Arc<Self> {
        Arc::default()
    }

    pub fn insert(&self, key: Vec<u8>, value: DeliverySender) -> Option<DeliverySender> {
        self.map.insert(key, value)
    }

    pub fn get<'s>(
        &'s self,
        swiss: &[u8],
    ) -> Option<dashmap::mapref::one::Ref<'s, Vec<u8>, DeliverySender>> {
        self.map.get(swiss)
    }

    pub fn remove(&self, swiss: &[u8]) -> Option<DeliverySender> {
        self.map.remove(swiss).map(|(_, v)| v)
    }
}
