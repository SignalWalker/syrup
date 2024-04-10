use crate::{
    de::{RecordFieldAccess, Visitor},
    ser::SerializeRecord,
    Deserialize, Serialize, Symbol,
};
use ed25519_dalek::{Signature, VerifyingKey};

mod __impl_vkey {
    use crate::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    #[syrup(crate = crate, name = "curve")]
    pub(super) struct Curve {
        #[syrup(as_symbol)]
        pub(super) kind: String,
    }
    #[derive(Serialize, Deserialize)]
    #[syrup(crate = crate, name = "flags")]
    pub(super) struct Flags {
        #[syrup(as_symbol)]
        pub(super) flags: String,
    }
    #[derive(Serialize, Deserialize)]
    #[syrup(crate = crate, name = "q")]
    pub(super) struct Q {
        #[syrup(with = crate::bytes::array)]
        pub(super) q: [u8; 32],
    }
    #[derive(Serialize, Deserialize)]
    #[syrup(crate = crate, name = "ecc")]
    pub(super) struct Ecc {
        pub(super) curve: Curve,
        pub(super) flags: Flags,
        pub(super) q: Q,
    }
}

impl Serialize for VerifyingKey {
    fn serialize<Ser: crate::ser::Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        __impl_vkey::Ecc {
            curve: __impl_vkey::Curve {
                kind: "Ed25519".to_owned(),
            },
            flags: __impl_vkey::Flags {
                flags: "eddsa".to_owned(),
            },
            q: __impl_vkey::Q {
                q: *self.as_bytes(),
            },
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for VerifyingKey {
    fn deserialize<D: crate::de::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let ecc = __impl_vkey::Ecc::deserialize(de)?;
        if ecc.curve.kind != "Ed25519" {
            todo!()
        }
        if ecc.flags.flags != "eddsa" {
            todo!()
        }
        Ok(Self::from_bytes(&ecc.q.q).unwrap())
    }
}

impl Serialize for Signature {
    /// Serialize as `<eddsa <r [u8]> <s [u8]>>`
    fn serialize<Ser: crate::ser::Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
        #[derive(Serialize)]
        #[syrup(crate = crate, name = "r")]
        struct __R<'r> {
            #[syrup(with = crate::bytes::array)]
            r: &'r [u8; 32],
        }
        #[derive(Serialize)]
        #[syrup(crate = crate, name = "s")]
        struct __S<'s> {
            #[syrup(with = crate::bytes::array)]
            s: &'s [u8; 32],
        }
        let mut rec = s.serialize_record("eddsa", Some(2))?;
        rec.serialize_field(&__R { r: self.r_bytes() })?;
        rec.serialize_field(&__S { s: self.s_bytes() })?;
        rec.end()
    }
}

impl<'de> Deserialize<'de> for Signature {
    /// Deserialize from `<eddsa <r [u8]> <s [u8]>>`
    fn deserialize<D: crate::de::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct __Visitor;
        impl<'de> Visitor<'de> for __Visitor {
            type Value = Signature;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("eddsa signature record")
            }

            fn visit_record<R: crate::de::RecordAccess<'de>>(
                self,
                rec: R,
            ) -> Result<Self::Value, R::Error> {
                #[derive(Deserialize)]
                #[syrup(crate = crate, name = "r")]
                struct __R {
                    #[syrup(with = crate::bytes::array)]
                    r: [u8; 32],
                }

                #[derive(Deserialize)]
                #[syrup(crate = crate, name = "s")]
                struct __S {
                    #[syrup(with = crate::bytes::array)]
                    s: [u8; 32],
                }

                let (mut rec, label) = rec.label::<Symbol<&'de str>>()?;
                if label.0 != "eddsa" {
                    todo!()
                }
                let r = rec.next_field::<__R>()?.unwrap().r;
                let s = rec.next_field::<__S>()?.unwrap().s;
                Ok(Self::Value::from_components(r, s))
            }
        }
        de.deserialize_record(__Visitor)
    }
}
