use crate::DynErr;
use std::io::{Read, Write};
use std::str;

#[derive(Debug)]
pub struct Frame<'a> {
    pub method: &'a str,
    pub arg: Argument<'a>,
}

#[derive(Debug)]
pub struct Argument<'a> {
    pub id: u32,
    pub data: &'a [u8],
}

impl<'a> Argument<'a> {
    pub fn encode(&self, writer: &mut impl Write) -> Result<(), DynErr> {
        writer.write_all(&self.id.to_be_bytes())?;
        writer.write_all(&self.data)?;
        Ok(())
    }
}

impl<'a> Frame<'a> {
    pub fn encode(&self, writer: &mut impl Write) -> Result<(), DynErr> {
        let method = self.method.as_bytes();
        let method_len: u8 = method.len().try_into()?;

        writer.write_all(&method_len.to_be_bytes())?;
        writer.write_all(method)?;
        self.arg.encode(writer)?;
        Ok(())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, DynErr> {
        let mut buf = vec![];
        self.encode(&mut buf)?;
        Ok(buf)
    }
}

pub fn get_slice<'de>(reader: &mut &'de [u8], len: usize) -> Result<&'de [u8], &'static str> {
    if len <= reader.len() {
        unsafe {
            let slice = reader.get_unchecked(..len);
            *reader = reader.get_unchecked(len..);
            Ok(slice)
        }
    } else {
        Err("insufficient bytes")
    }
}

#[derive(Debug)]
pub struct Request {
    buf: Box<[u8]>,
    method: Box<str>,
    id: u32,
    data_offset: u16,
}

impl Request {
    pub fn parse(buf: Box<[u8]>) -> Result<Request, DynErr> {
        let reader = &mut &buf[..];

        let len: usize = get_slice(reader, 1)?[0].into();
        let method = str::from_utf8(get_slice(reader, len)?)?.into();

        let mut id = [0; 4];
        reader.read_exact(&mut id)?;
        let id = u32::from_be_bytes(id);

        let data_offset = (buf.len() - reader.len()) as u16;
        Ok(Self {
            buf,
            method,
            id,
            data_offset,
        })
    }
}

impl Request {
    #[inline]
    pub fn method(&self) -> &str {
        &self.method
    }

    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.buf[self.data_offset.into()..]
    }
}

pub struct Notify {
    buf: Box<[u8]>,
    method: Box<str>,
    data_offset: u16,
}

impl From<Request> for Notify {
    fn from(req: Request) -> Self {
        Self {
            buf: req.buf,
            method: req.method,
            data_offset: req.data_offset,
        }
    }
}

impl Notify {
    #[inline]
    pub fn method(&self) -> &str {
        &self.method
    }

    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.buf[self.data_offset.into()..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_frame() {
        let req = Frame {
            method: &"1".repeat(256),
            arg: Argument {
                id: 42,
                data: &[1, 2],
            },
        };

        // Error Reason: method name too long
        assert!(req.to_bytes().is_err());

        let req: Frame<'_> = Frame {
            method: "foo",
            arg: Argument {
                id: 42,
                data: &[1, 2],
            },
        };
        let bytes = req.to_bytes().unwrap();
        let frame = Request::parse(bytes.into()).unwrap();

        assert_eq!(frame.method(), "foo");
        assert_eq!(frame.id(), 42);
        assert_eq!(frame.data(), &[1, 2]);
    }
}
