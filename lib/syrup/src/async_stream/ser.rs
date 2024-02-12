use crate::{
    ser::{SerializeDict, SerializeRecord, SerializeSeq, SerializeSet, Serializer},
    Error,
};
use futures::{io::WriteAll, AsyncWrite, AsyncWriteExt, Future};
use ibig::UBig;
use std::{
    any::Any,
    pin::{pin, Pin},
};

pub struct SerializeFuture<'w, Writer: ?Sized> {
    write: WriteAll<'w, Writer>,
    buf: Pin<Box<dyn Any>>,
}

impl<'w, Writer: AsyncWrite + Unpin + ?Sized> Future for SerializeFuture<'w, Writer> {
    type Output = <WriteAll<'w, Writer> as Future>::Output;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        Future::poll(pin!(&mut self.write), cx)
    }
}

fn write_all<'w, W: AsyncWrite + Unpin>(
    writer: &'w mut W,
    buf: Vec<u8>,
) -> Result<SerializeFuture<'w, W>, Error<'static>> {
    let buf: Pin<Box<dyn Any>> = Box::<dyn Any>::into_pin(Box::new(buf));
    let buf_ref = unsafe {
        std::mem::transmute::<&Vec<u8>, &'w Vec<u8>>(buf.downcast_ref::<Vec<u8>>().unwrap())
    };
    let write = writer.write_all(buf_ref);
    Ok(SerializeFuture { write, buf })
}

fn write_all_slice<'w, W: AsyncWrite + Unpin>(
    writer: &'w mut W,
    buf: &'static [u8],
) -> Result<SerializeFuture<'w, W>, Error<'static>> {
    let write = writer.write_all(buf);
    Ok(SerializeFuture {
        write,
        buf: Box::into_pin(Box::new(buf)),
    })
}

pub struct AsyncWriteSetSerializer<'ser, 'w, W> {
    ser: &'ser mut AsyncWriteSerializer<'w, W>,
}

impl<'w, 'ser: 'w, W: AsyncWrite + Unpin> SerializeSet for AsyncWriteSetSerializer<'ser, 'w, W> {
    type Ok = <&'ser mut AsyncWriteSerializer<'w, W> as Serializer>::Ok;
    type Error = <&'ser mut AsyncWriteSerializer<'w, W> as Serializer>::Error;

    fn serialize_element<El: crate::Serialize + ?Sized>(
        &mut self,
        el: &El,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

pub struct AsyncWriteDictSerializer<'ser, 'w, W> {
    ser: &'ser mut AsyncWriteSerializer<'w, W>,
}

impl<'w, 'ser: 'w, W: AsyncWrite + Unpin> SerializeDict for AsyncWriteDictSerializer<'ser, 'w, W> {
    type Ok = <&'ser mut AsyncWriteSerializer<'w, W> as Serializer>::Ok;
    type Error = <&'ser mut AsyncWriteSerializer<'w, W> as Serializer>::Error;

    fn serialize_entry<Ke: crate::Serialize + ?Sized, Va: crate::Serialize + ?Sized>(
        &mut self,
        key: &Ke,
        value: &Va,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

pub struct AsyncWriteSerializer<'writer, Writer> {
    pub writer: &'writer mut Writer,
}

impl<'w, 'ser: 'w, W: AsyncWrite + Unpin> SerializeSeq for &'ser mut AsyncWriteSerializer<'w, W> {
    type Ok = <Self as Serializer>::Ok;
    type Error = <Self as Serializer>::Error;

    fn serialize_element<El: crate::Serialize + ?Sized>(
        &mut self,
        el: &El,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'w, 'ser: 'w, W: AsyncWrite + Unpin> SerializeRecord
    for &'ser mut AsyncWriteSerializer<'w, W>
{
    type Ok = <Self as Serializer>::Ok;
    type Error = <Self as Serializer>::Error;

    fn serialize_field<Fi: crate::Serialize + ?Sized>(
        &mut self,
        fi: &Fi,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

macro_rules! serialize_int {
    ($serialize_fn:ident, $Int:ty, $UInt:ty) => {
        fn $serialize_fn(self, v: $Int) -> Result<Self::Ok, Self::Error> {
            let mut buf = v
                .checked_abs()
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| {
                    (<$UInt>::try_from(<$Int>::MAX).unwrap() + <$UInt>::from(1u8)).to_string()
                })
                .into_bytes();
            buf.push(if v < 0 { b'-' } else { b'+' });
            write_all(self.writer, buf)
        }
    };
}

macro_rules! serialize_uint {
    ($serialize_fn:ident, $Int:ty) => {
        fn $serialize_fn(self, v: $Int) -> Result<Self::Ok, Self::Error> {
            let mut buf = v.to_string().into_bytes();
            buf.push(b'+');
            write_all(self.writer, buf)
        }
    };
}

impl<'w, 'ser: 'w, Writer: AsyncWrite + Unpin> Serializer
    for &'ser mut AsyncWriteSerializer<'w, Writer>
{
    type Ok = SerializeFuture<'w, Writer>;
    type Error = Error<'static>;

    type SerializeDict = AsyncWriteDictSerializer<'ser, 'w, Writer>;

    type SerializeSeq = Self;

    type SerializeRecord = Self;

    type SerializeSet = AsyncWriteSetSerializer<'ser, 'w, Writer>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        write_all_slice(
            self.writer,
            match v {
                true => b"t",
                false => b"f",
            },
        )
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        let mut buf = vec![b'F'];
        buf.extend_from_slice(&v.to_be_bytes());
        write_all(self.writer, buf)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        let mut buf = vec![b'F'];
        buf.extend_from_slice(&v.to_be_bytes());
        write_all(self.writer, buf)
    }

    serialize_int!(serialize_i8, i8, u16);
    serialize_int!(serialize_i16, i16, u32);
    serialize_int!(serialize_i32, i32, u64);
    serialize_int!(serialize_i64, i64, u128);
    serialize_int!(serialize_i128, i128, UBig);
    serialize_int!(serialize_isize, isize, u128);

    serialize_uint!(serialize_u8, u8);
    serialize_uint!(serialize_u16, u16);
    serialize_uint!(serialize_u32, u32);
    serialize_uint!(serialize_u64, u64);
    serialize_uint!(serialize_u128, u128);
    serialize_uint!(serialize_usize, usize);

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        let mut buf = v.len().to_string().into_bytes();
        buf.push(b'"');
        buf.extend_from_slice(v.as_bytes());
        write_all(self.writer, buf)
    }

    fn serialize_sym(self, v: &str) -> Result<Self::Ok, Self::Error> {
        let mut buf = v.len().to_string().into_bytes();
        buf.push(b'\'');
        buf.extend_from_slice(v.as_bytes());
        write_all(self.writer, buf)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let mut buf = v.len().to_string().into_bytes();
        buf.push(b':');
        buf.extend_from_slice(v);
        write_all(self.writer, buf)
    }

    fn serialize_dictionary(self, len: Option<usize>) -> Result<Self::SerializeDict, Self::Error> {
        todo!()
    }

    fn serialize_sequence(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        todo!()
    }

    fn serialize_record(
        self,
        name: &str,
        len: Option<usize>,
    ) -> Result<Self::SerializeRecord, Self::Error> {
        todo!()
    }

    fn serialize_set(self, len: Option<usize>) -> Result<Self::SerializeSet, Self::Error> {
        todo!()
    }

    unsafe fn serialize_raw(self, data: &[u8]) -> Result<Self::Ok, Self::Error> {
        write_all(self.writer, data.to_vec())
    }
}
