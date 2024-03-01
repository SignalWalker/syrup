use syrup::{de::RecordFieldAccess, Deserialize, Serialize};

mod start_session;
pub use start_session::*;

mod abort;
pub use abort::*;

mod import_export {
    use syrup::{
        de::{RecordFieldAccess, Visitor},
        Deserialize, Serialize, Symbol,
    };

    #[derive(Clone, Serialize, Deserialize)]
    #[syrup(name = "desc:export")]
    pub struct DescExport {
        pub position: u64,
    }

    impl From<u64> for DescExport {
        fn from(position: u64) -> Self {
            Self { position }
        }
    }

    impl std::fmt::Debug for DescExport {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    #[syrup(name = "desc:import-object")]
    pub struct DescImportObject {
        pub position: u64,
    }

    impl std::fmt::Debug for DescImportObject {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }

    impl From<u64> for DescImportObject {
        fn from(position: u64) -> Self {
            Self { position }
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    #[syrup(name = "desc:import-promise")]
    pub struct DescImportPromise {
        pub position: u64,
    }

    impl std::fmt::Debug for DescImportPromise {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }

    impl From<u64> for DescImportPromise {
        fn from(position: u64) -> Self {
            Self { position }
        }
    }

    #[derive(Clone)]
    pub enum DescImport {
        Object(DescImportObject),
        Promise(DescImportPromise),
    }

    impl std::fmt::Debug for DescImport {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                DescImport::Object(o) => o.fmt(f),
                DescImport::Promise(p) => p.fmt(f),
            }
        }
    }

    impl From<DescImportObject> for DescImport {
        fn from(value: DescImportObject) -> Self {
            Self::Object(value)
        }
    }

    impl From<DescImportPromise> for DescImport {
        fn from(value: DescImportPromise) -> Self {
            Self::Promise(value)
        }
    }

    impl<'de> Deserialize<'de> for DescImport {
        fn deserialize<D: syrup::de::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            struct __Visitor;
            impl<'de> Visitor<'de> for __Visitor {
                type Value = DescImport;

                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "either desc:import-object or desc:import-promise")
                }

                fn visit_record<R: syrup::de::RecordAccess<'de>>(
                    self,
                    rec: R,
                ) -> Result<Self::Value, R::Error> {
                    let (mut rec, label) = rec.label::<Symbol<&str>>()?;
                    match label.0 {
                        "desc:import-promise" => Ok(DescImport::Promise(DescImportPromise {
                            position: rec.next_field()?.unwrap(),
                        })),
                        "desc:import-object" => Ok(DescImport::Object(DescImportObject {
                            position: rec.next_field()?.unwrap(),
                        })),
                        _ => todo!(),
                    }
                }
            }
            de.deserialize_record(__Visitor)
        }
    }

    impl Serialize for DescImport {
        fn serialize<Ser: syrup::ser::Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
            match self {
                DescImport::Object(o) => o.serialize(s),
                DescImport::Promise(p) => p.serialize(s),
            }
        }
    }
}
pub use import_export::*;

mod deliver {
    use super::{DescExport, DescImport};
    use syrup::{Deserialize, Serialize};

    #[derive(Clone, Serialize, Deserialize)]
    #[syrup(name = "op:deliver-only")]
    pub struct OpDeliverOnly<Arg> {
        pub to_desc: DescExport,
        pub args: Vec<Arg>,
    }

    impl<Arg: syrup::Serialize> std::fmt::Debug for OpDeliverOnly<Arg> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }

    impl<Arg> OpDeliverOnly<Arg> {
        pub fn new(position: u64, args: Vec<Arg>) -> Self {
            Self {
                to_desc: position.into(),
                args,
            }
        }
    }

    impl OpDeliverOnly<syrup::RawSyrup> {
        pub fn from_ident_args<'arg, Arg: Serialize + 'arg>(
            position: u64,
            ident: impl AsRef<str>,
            args: impl IntoIterator<Item = &'arg Arg>,
        ) -> Result<Self, syrup::Error<'static>> {
            let mut serialized_args = syrup::raw_syrup![&syrup::Symbol::from(ident.as_ref()),];
            serialized_args.extend(args.into_iter().map(syrup::RawSyrup::from_serialize));
            Ok(Self::new(position, serialized_args))
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    #[syrup(name = "op:deliver")]
    pub struct OpDeliver<Arg> {
        pub to_desc: DescExport,
        pub args: Vec<Arg>,
        pub answer_pos: Option<u64>,
        pub resolve_me_desc: DescImport,
    }

    impl<Arg: syrup::Serialize> std::fmt::Debug for OpDeliver<Arg> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }

    impl<Arg> OpDeliver<Arg> {
        pub fn new(
            position: u64,
            args: Vec<Arg>,
            answer_pos: Option<u64>,
            resolve_me_desc: DescImport,
        ) -> Self {
            Self {
                to_desc: position.into(),
                args,
                answer_pos,
                resolve_me_desc,
            }
        }
    }

    impl OpDeliver<syrup::RawSyrup> {
        pub fn from_ident_args<'arg, Arg: Serialize + 'arg>(
            position: u64,
            ident: impl AsRef<str>,
            args: impl IntoIterator<Item = &'arg Arg>,
            answer_pos: Option<u64>,
            resolve_me_desc: DescImport,
        ) -> Result<Self, syrup::Error<'static>> {
            let mut serialized_args = syrup::raw_syrup![&syrup::Symbol::from(ident.as_ref()),];
            serialized_args.extend(args.into_iter().map(syrup::RawSyrup::from_serialize));
            Ok(Self::new(
                position,
                serialized_args,
                answer_pos,
                resolve_me_desc,
            ))
        }
    }
}
pub use deliver::*;

mod handoff {
    use super::PublicKey;
    use crate::locator::NodeLocator;
    use syrup::{Deserialize, Serialize};

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[syrup(name = "desc:handoff-give", deserialize_bound = HKey: PartialEq + Eq + std::hash::Hash + Deserialize<'__de>; HVal: Deserialize<'__de>)]
    pub struct DescHandoffGive<HKey, HVal> {
        pub receiver_key: PublicKey,
        pub exporter_location: NodeLocator<HKey, HVal>,
        #[syrup(with = syrup::bytes::vec)]
        pub session: Vec<u8>,
        #[syrup(with = syrup::bytes::vec)]
        pub gifter_side: Vec<u8>,
        #[syrup(with = syrup::bytes::vec)]
        pub gift_id: Vec<u8>,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[syrup(name = "desc:handoff-receive", deserialize_bound = HKey: PartialEq + Eq + std::hash::Hash + Deserialize<'__de>; HVal: Deserialize<'__de>)]
    pub struct DescHandoffReceive<HKey, HVal> {
        #[syrup(with = syrup::bytes::vec)]
        pub receiving_session: Vec<u8>,
        #[syrup(with = syrup::bytes::vec)]
        pub receiving_side: Vec<u8>,
        pub handoff_count: u64,
        pub signed_give: DescHandoffGive<HKey, HVal>,
    }
}
pub use handoff::*;

#[derive(Clone)]
pub enum Operation<Inner> {
    DeliverOnly(OpDeliverOnly<Inner>),
    Deliver(OpDeliver<Inner>),
    // Pick(OpPick),
    Abort(OpAbort),
    // Listen(OpListen),
    // GcExport(OpGcExport),
    // GcAnswer(OpGcAnswer),
}

impl<Inner: syrup::Serialize> std::fmt::Debug for Operation<Inner> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeliverOnly(d) => d.fmt(f),
            Self::Deliver(d) => d.fmt(f),
            Self::Abort(a) => a.fmt(f),
        }
    }
}

impl<'de, Inner: syrup::Deserialize<'de>> syrup::Deserialize<'de> for Operation<Inner> {
    fn deserialize<D: syrup::de::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct __Visitor<Inner>(std::marker::PhantomData<Inner>);
        impl<'de, Inner: Deserialize<'de>> syrup::de::Visitor<'de> for __Visitor<Inner> {
            type Value = Operation<Inner>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("op:start-session, op:deliver-only, op:deliver, op:pick, op:abort, op:listen, op:gc-export, or op:gc-answer")
            }

            fn visit_record<R: syrup::de::RecordAccess<'de>>(
                self,
                rec: R,
            ) -> Result<Self::Value, R::Error> {
                let (mut rec, label) = rec.label::<syrup::Symbol<&str>>()?;
                match label.0 {
                    "op:deliver-only" => Ok(Operation::DeliverOnly(OpDeliverOnly {
                        to_desc: rec.next_field()?.unwrap(),
                        args: rec.next_field()?.unwrap(),
                    })),
                    "op:deliver" => Ok(Operation::Deliver(OpDeliver {
                        to_desc: rec.next_field()?.unwrap(),
                        args: rec.next_field()?.unwrap(),
                        answer_pos: rec.next_field()?.unwrap(),
                        resolve_me_desc: rec.next_field()?.unwrap(),
                    })),
                    "op:pick" => todo!("op:pick"),
                    "op:abort" => Ok(Operation::Abort(OpAbort {
                        reason: rec.next_field()?.unwrap(),
                    })),
                    "op:listen" => todo!("op:listen"),
                    "op:gc-export" => todo!("op:gc-export"),
                    "op:gc-answer" => todo!("op:gc-answer"),
                    _ => Err(todo!("unrecognized operation")),
                }
            }
        }
        de.deserialize_record(__Visitor(std::marker::PhantomData))
    }
}
