//! Config file processing.
//!
//! Borg uses a small subset of the Python config file parser.  The specific language used here
//! seems to be under-specified, so we'll implement enough to parse what we find in the files now.

// TODO: Need to handle blank lines it likes to insert.

use data_encoding::base64;
use nom::{alpha, digit, hex_digit, line_ending, tab};

use std::str::{self, FromStr};

#[derive(Debug)]
pub enum Value {
    Int(u64),
    Hex(String),
    Text(String),
    Base64(Vec<u8>),
}

fn make_hex(bytes: &[u8]) -> Value {
    Value::Hex(String::from_utf8(bytes.to_owned()).unwrap())
}

fn idchar(chr: u8) -> bool {
    (chr >= b'a' && chr <= b'z') || chr == b'_'
}

fn is_base64(chr: u8) -> bool {
    (chr >= b'a' && chr <= b'z') ||
    (chr >= b'A' && chr <= b'Z') ||
    (chr >= b'0' && chr <= b'9') ||
    chr == b'+' || chr == b'/' || chr == b'='
}

fn from_base64(lines: Vec<&[u8]>) -> Value {
    let mut buf = vec![];
    for line in lines {
        buf.extend_from_slice(line);
    }
    Value::Base64(base64::decode(&buf).unwrap())
}

// The config files used here are more restricted.
// We take the [asdf] line, and just make it into a special entry with no key, and value as the
// identifier.

named!(integer<u64>,
       map_res!(
           map_res!(
               digit,
               str::from_utf8),
            FromStr::from_str));

named!(value<Value>, alt!(
        map!(integer, Value::Int) |
        map!(hex_digit, make_hex) |
        map!(base64, from_base64)));

// Base-64 values, spanning multiple lines.  The ConfigParser is pretty flexible, but we'll, for
// now, just handle the newline/tab delimiter.
named!(base64<&[u8], Vec<&[u8]>, u32>, separated_nonempty_list_complete!(
    pair!(line_ending, tab),
    b64line));

// This has to be separated out, because of a type-inference problem.
named!(b64line<&[u8]>, take_while1!(is_base64));

named!(section<(String, Value)>,
    map!(delimited!(tag!("["), alpha, tag!("]")),
        |x: &[u8]| ("".to_string(), Value::Text(String::from_utf8(x.to_owned()).unwrap()))));

named!(entry<(String, Value)>,
   separated_pair!(
       map_res!(take_while1!(idchar), |x: &[u8]| String::from_utf8(x.to_owned())),
       tag!(" = "),
       value));

named!(pub entries<Vec<(String, Value)> >,
    terminated!(many0!(terminated!(alt!(entry | section), many1!(line_ending))), eof!()));
