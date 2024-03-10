use rexa::{captp::AbstractCapTpSession, impl_object};

struct Obj<T>(T);

#[impl_object()]
impl<T> Obj<T>
where
    T: Sync,
{
    #[deliver_only()]
    fn test_deliver_only(&self, _: String) -> Result<(), std::io::Error> {
        todo!()
    }

    #[deliver()]
    async fn test_deliver(&self, _: u64) -> Result<u64, String> {
        todo!()
    }

    #[deliver()]
    fn test_session(&self, session: &(dyn AbstractCapTpSession + Sync)) -> Result<u64, String> {
        todo!()
    }

    #[deliver_only(fallback)]
    fn deliver_only_fallback(
        &self,
        args: Vec<syrup::Item>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        todo!()
    }
}
