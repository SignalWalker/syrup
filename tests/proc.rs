use rexa::{captp::AbstractCapTpSession, impl_object};

struct Obj<T>(T);

#[impl_object()]
impl<T> Obj<T>
where
    T: Sync,
{
    #[object(deliver_only)]
    fn test_deliver_only(&self, _: String) -> Result<(), std::io::Error> {
        todo!()
    }

    #[object(deliver)]
    async fn test_deliver(&self, _: u64) -> Result<(), std::io::Error> {
        todo!()
    }

    #[object(deliver)]
    fn test_session(
        &self,
        #[object(session)] session: &(dyn AbstractCapTpSession + Sync),
    ) -> Result<(), std::io::Error> {
        todo!()
    }
}
