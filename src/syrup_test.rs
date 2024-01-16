use syrup::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[syrup(name = "syrup-test")]
pub struct SyrupTest<T> {
    f: String,
    #[syrup(deserialize_with = test_des)]
    t: T,
}

fn test_des<'i, T: syrup::de::Deserialize<'i>, Des: syrup::de::Deserializer<'i>>(
    des: Des,
) -> Result<T, Des::Error> {
    todo!()
}
