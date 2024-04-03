#![allow(dead_code, unused)]
use std::{
    io::{Error, ErrorKind},
    vec,
};

use async_recursion::async_recursion;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, BufReader};

/// up to 512 MB in length
const RESP_MAX_SIZE: i64 = 512 * 1024 * 1024;
const CRLF_BYTES: &'static [u8] = b"\r\n";
const NULL_BYTES: &'static [u8] = b"$-1\r\n";
const NULL_ARRAY_BYTES: &'static [u8] = b"*-1\r\n";

pub struct RespParser<R> {
    pub reader: BufReader<R>,
    pub buf_bulk: bool,
}

impl<R> RespParser<R>
where
    R: AsyncBufRead + Unpin + Send,
{
    pub fn new(reader: R) -> Self {
        RespParser {
            reader: BufReader::new(reader),
            buf_bulk: false,
        }
    }

    pub fn with_buf_bulk(reader: BufReader<R>) -> Self {
        RespParser {
            reader,
            buf_bulk: true,
        }
    }

    #[async_recursion]
    pub async fn decode(&mut self) -> Result<RespDT, Box<dyn std::error::Error>> {
        let mut res: Vec<u8> = Vec::new();
        self.reader.read_until(b'\n', &mut res).await?;
        let fb = res[0];
        let len = res.len();
        if len == 0 {
            return Err(Error::new(ErrorKind::UnexpectedEof, "unexpected EOF").into());
        }
        if len < 3 {
            return Err(Error::new(ErrorKind::InvalidInput, format!("too short: {}", len)).into());
        }
        if !is_crlf(res[len - 2], res[len - 1]) {
            return Err(
                Error::new(ErrorKind::InvalidInput, format!("invalid CRLF: {:?}", res)).into(),
            );
        }
        let bytes = res[1..len - 2].as_ref();
        match fb {
            b'+' => parse_string(bytes).map(|s| RespDT::SimpleString(s)),
            b'-' => parse_string(bytes).map(|s| RespDT::SimpleError(s)),
            b':' => parse_integer(bytes).map(|i| RespDT::Integer(i)),
            b'$' => {
                let data_length = parse_integer(bytes)?;
                if data_length == -1 {
                    return Ok(RespDT::Null);
                }
                if data_length < -1 || data_length > RESP_MAX_SIZE {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("invalid bulk string length: {}", data_length),
                    )
                    .into());
                }
                let mut buf = vec![0; (data_length + 2) as usize];
                self.reader.read_exact(&mut buf).await?;
                if !is_crlf(buf[buf.len() - 2], buf[buf.len() - 1]) {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("invalid CRLF: {:?}", buf),
                    )
                    .into());
                }
                buf.truncate(data_length as usize);
                if self.buf_bulk {
                    return Ok(RespDT::BufBulk(buf));
                }
                return parse_string(&buf).map(|s| RespDT::Bulk(s));
            }
            b'*' => {
                let data_length = parse_integer(bytes)?;
                if data_length == -1 {
                    return Ok(RespDT::NullArray);
                }
                if data_length < -1 || data_length > RESP_MAX_SIZE {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("invalid array length: {}", data_length),
                    )
                    .into());
                }
                let mut arr = Vec::with_capacity(data_length as usize);
                for _ in 0..data_length {
                    arr.push(self.decode().await?);
                }
                Ok(RespDT::Array(arr))
            }
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                format!("invalid RESP type: {}", fb),
            )
            .into()),
        }
    }
}

#[inline]
fn is_crlf(a: u8, b: u8) -> bool {
    a == b'\r' && b == b'\n'
}

#[inline]
fn parse_string(bytes: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    String::from_utf8(bytes.to_vec()).map_err(|err| Error::new(ErrorKind::InvalidData, err).into())
}

#[inline]
fn parse_integer(bytes: &[u8]) -> Result<i64, Box<dyn std::error::Error>> {
    String::from_utf8(bytes.to_vec())?
        .parse::<i64>()
        .map_err(|err| Error::new(ErrorKind::InvalidData, err).into())
}

#[inline]
pub fn encode(resp: &RespDT) -> Result<String, Box<dyn std::error::Error>> {
    let mut res: Vec<u8> = Vec::new();
    buf_encode(resp, &mut res);
    String::from_utf8(res).map_err(|err| Error::new(ErrorKind::InvalidData, err).into())
}

#[inline]
pub fn encode_raw(resp: &RespDT) -> Vec<u8> {
    let mut res: Vec<u8> = Vec::new();
    buf_encode(resp, &mut res);
    res
}

#[inline]
pub fn encode_slice(slice: &[&str]) -> Vec<u8> {
    let array: Vec<RespDT> = slice
        .iter()
        .map(|string| RespDT::Bulk(string.to_string()))
        .collect();
    let mut res: Vec<u8> = Vec::new();
    buf_encode(&RespDT::Array(array), &mut res);
    res
}

#[inline]
fn buf_encode(resp: &RespDT, buf: &mut Vec<u8>) {
    match resp {
        RespDT::Null => buf.extend_from_slice(NULL_BYTES),
        RespDT::NullArray => buf.extend_from_slice(NULL_ARRAY_BYTES),
        RespDT::SimpleString(s) => {
            buf.extend_from_slice(b"+");
            buf.extend_from_slice(s.as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
        }
        RespDT::SimpleError(s) => {
            buf.extend_from_slice(b"-");
            buf.extend_from_slice(s.as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
        }
        RespDT::Integer(i) => {
            buf.extend_from_slice(b":");
            buf.extend_from_slice(i.to_string().as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
        }
        RespDT::Bulk(s) => {
            buf.extend_from_slice(b"$");
            buf.extend_from_slice(s.len().to_string().as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
            buf.extend_from_slice(s.as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
        }
        RespDT::BufBulk(data) => {
            buf.extend_from_slice(b"$");
            buf.extend_from_slice(data.len().to_string().as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
            buf.extend_from_slice(&data);
            buf.extend_from_slice(CRLF_BYTES);
        }
        RespDT::Array(arr) => {
            buf.extend_from_slice(b"*");
            buf.extend_from_slice(arr.len().to_string().as_bytes());
            buf.extend_from_slice(CRLF_BYTES);
            for resp in arr {
                buf_encode(resp, buf);
            }
        }
    }
}

#[derive(Debug)]
pub enum RespDT {
    SimpleString(String),
    SimpleError(String),
    Integer(i64),
    Bulk(String),
    BufBulk(Vec<u8>),
    Array(Vec<RespDT>),
    NullArray,
    Null,
    // Booleans(char),
    // Doubles(char),
    // BigNums(char),
    // BulkError(char),
    // VerbatimString(char),
    // Maps(char),
    // Sets(char),
    // Pushes(char),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_echo() {
        let input = b"*2\r\n$4\r\necho\r\n$3\r\nhey\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_ping() {
        let input = b"*1\r\n$4\r\nping\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }
    #[tokio::test]
    async fn test_parse_simple_string() {
        let input = b"+OK\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_simple_error() {
        let input = b"-ERR unknown command 'asdf'\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_simple_error_with_wrongtype() {
        let input = b"-WRONGTYPE Operation against a key holding the wrong kind of value\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_integer() {
        let input = b":42\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_intege_negative() {
        let input = b":-6742\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_bulk_string() {
        let input = b"$5\r\nHello\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_bulk_string_empty() {
        let input = b"$0\r\n\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_buf_bulk_string() {
        let input = b"$5\r\nHello\r\n";
        let mut parser = RespParser::with_buf_bulk(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_buf_bulk_string_empty() {
        let input = b"$0\r\n\r\n";
        let mut parser = RespParser::with_buf_bulk(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_null_array() {
        let input = b"*-1\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_array_empty() {
        let input = b"*0\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_array_two_strs() {
        let input = b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_array_three_ints() {
        let input = b"*3\r\n:1\r\n:2\r\n:3\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_array_mix() {
        let input = b"*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$5\r\nhello\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_array_nested() {
        let input = b"*2\r\n*3\r\n:1\r\n:2\r\n:3\r\n*2\r\n+Hello\r\n-World\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn test_parse_array_command() {
        let input = b"*2\r\n$4\r\nLLEN\r\n$6\r\nmylist\r\n";
        let mut parser = RespParser::new(BufReader::new(&input[..]));
        let r = parser.decode().await;
        assert!(r.is_ok());
        dbg!(r);
    }

    #[tokio::test]
    async fn fn_encode_slice() {
        let array = ["SET", "a", "1"];
        assert_eq!(
            String::from_utf8(encode_slice(&array)).unwrap(),
            "*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n"
        );

        let array = vec!["SET", "a", "1"];
        assert_eq!(
            String::from_utf8(encode_slice(&array)).unwrap(),
            "*3\r\n$3\r\nSET\r\n$1\r\na\r\n$1\r\n1\r\n"
        );
    }

    #[tokio::test]
    async fn fn_encode_pong() {
        let r = encode(&RespDT::SimpleString("PONG".to_string()));
        assert!(r.is_ok());
        dbg!(r.unwrap());
    }
}
