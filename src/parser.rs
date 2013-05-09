extern mod std;

use http_parser;
use http_parser::{http_parser_init, http_parser_execute, HTTP_REQUEST};
use core::ptr::{null, to_unsafe_ptr};
use core::libc::{c_int, c_char, size_t, c_void, c_uint};
use core::cast::{reinterpret_cast};
use headers::*;

pub struct Parser {
  parser: ~http_parser::http_parser,
  result: ParseResult,
  offset: uint
}

#[deriving(Eq)]
enum Method {
  DELETE, GET, HEAD, POST, PUT, CONNECT, OPTIONS,
  TRACE, COPY, LOCK, MKCOL, MOVE, PROPFIND, PROPPATCH,
  SEARCH, UNLOCK, REPORT, MKACTIVITY, CHECKOUT, MERGE,
  MSEARCH, NOTIFY, SUBSCRIBE, UNSUBSCRIBE, PATCH, PURGE
}

struct ParseResult {
  url: Option<~str>,
  method: Option<Method>,
  headers: HeaderMap,
  partial_header_field: Option<~str>,
  partial_header_value: Option<~str>
}

impl Parser {
  pub fn parse(&self, input: &str) -> Parser {
    let mut result = self.result.clone();

    let s = http_parser::Struct_http_parser_settings {
      on_message_begin: null(),
      on_url: on_url,
      on_status_complete: null(),
      on_header_field: on_header_field,
      on_header_value: on_header_value,
      on_headers_complete: on_headers_complete,
      on_body: null(),
      on_message_complete: null(),
    };

    let mut p = ~*self.parser;
    let mut offset = 0;
    p.data = to_unsafe_ptr(&result) as *c_void;

    do str::as_c_str(input) |buf| {
      unsafe {
        offset = http_parser_execute(to_unsafe_ptr(p),
                                     &s,
                                     buf,
                                     input.len() as u64);
      }
    }

    Parser {
      parser: p,
      result: result,
      offset: offset as uint
    }
  }

  pub fn finish(&self) -> Parser {
    self.parse("")
  }

  pub fn success(&self) -> bool {
    self.errno() == http_parser::HPE_OK
  }

  fn error_name(&self) -> ~str {
    unsafe {
      let c_str = http_parser::http_errno_name(self.errno());
      str::raw::from_c_str(c_str)
    }
  }

  fn error_description(&self) -> ~str {
    unsafe {
      let c_str = http_parser::http_errno_description(self.errno());
      str::raw::from_c_str(c_str)
    }
  }

  priv fn errno(&self) -> u32 {
    (self.parser.http_errno_upgrade & 0x7F) as u32
  }

  pub fn upgrade(&self) -> bool {
    (self.parser.http_errno_upgrade & 0x80) == 0x80
  }
}

impl ParseResult {
  fn header(&self, name: &str) -> Option<~str> {
    self.headers.get_header(name)
  }

  fn new() -> ParseResult {
    ParseResult {
       url: None,
       method: None,
       headers: HeaderMap::new(),
       partial_header_field: None,
       partial_header_value: None
    }
  }
}

impl Clone for ParseResult {
  fn clone(&self) -> ParseResult {
    ParseResult {
       url: self.url.clone(),
       method: self.method,
       headers: copy_headers(&self.headers),
       partial_header_field: self.partial_header_field.clone(),
       partial_header_value: self.partial_header_value.clone()
    }
  }
}

fn copy_headers(headers: &HeaderMap) -> HeaderMap {
  let mut copied = HeaderMap::new();

  for headers.each_key() |k| {
    let v = headers.find(k).expect("Invalid key while iterating over headers");
    copied.set_header(*k, *v);
  }

  copied
}


fn result_in_callback(p: *http_parser::Struct_http_parser) -> &mut ParseResult {
  unsafe { reinterpret_cast(&(*p).data) }
}

fn string_in_callback(at: *u8, length: size_t) -> ~str {
  unsafe { str::raw::from_c_str_len(at as *c_char, length as uint) }
}

extern fn on_url(p: *http_parser::Struct_http_parser,
                 at: *u8,
                 length: size_t) -> c_int {
  let url = string_in_callback(at, length);
  let result = result_in_callback(p);

  let mut current_url: Option<~str> = None;
  current_url <-> result.url;
  let new_url = current_url.get_or_default(~"") + url;
  result.url = Some(new_url);

  0
}

fn complete_partial_header(result: &mut ParseResult) {
  if result.partial_header_value.is_none() { return; }

  let mut new_field = None;
  let mut new_value = None;

  new_field <-> result.partial_header_field;
  new_value <-> result.partial_header_value;

  result.headers.set_header(
    new_field.expect("Got header value without header name!"),
    new_value.expect("Header value lied about is_some()!"));
}

extern fn on_header_field(p: *http_parser::Struct_http_parser,
                          at: *u8,
                          length: size_t) -> c_int {
  let result = result_in_callback(p);
  complete_partial_header(result);

  let field = string_in_callback(at, length);

  let mut current_field: Option<~str> = None;

  current_field <-> result.partial_header_field;
  result.partial_header_field = Some(current_field.get_or_default(~"") + field);

  0
}

extern fn on_header_value(p: *http_parser::Struct_http_parser,
                          at: *u8,
                          length: size_t) -> c_int {
  let result = result_in_callback(p);
  let value = string_in_callback(at, length);
  let mut current_value: Option<~str> = None;

  current_value <-> result.partial_header_value;
  result.partial_header_value = Some(current_value.get_or_default(~"") + value);

  0
}

extern fn on_headers_complete(p: *http_parser::Struct_http_parser)
                              -> c_int {
  let result = result_in_callback(p);
  complete_partial_header(result);

  let raw_method = unsafe { (*p).method as c_uint };

  result.method = http_method_const_to_enum(raw_method);

  0
}

fn http_method_const_to_enum(raw_method: c_uint) -> Option<Method> {
  match raw_method {
    http_parser::HTTP_DELETE => Some(DELETE),
    http_parser::HTTP_GET => Some(GET),
    http_parser::HTTP_HEAD => Some(HEAD),
    http_parser::HTTP_POST => Some(POST),
    http_parser::HTTP_PUT => Some(PUT),
    http_parser::HTTP_CONNECT => Some(CONNECT),
    http_parser::HTTP_OPTIONS => Some(OPTIONS),
    http_parser::HTTP_TRACE => Some(TRACE),
    http_parser::HTTP_COPY => Some(COPY),
    http_parser::HTTP_LOCK => Some(LOCK),
    http_parser::HTTP_MKCOL => Some(MKCOL),
    http_parser::HTTP_MOVE => Some(MOVE),
    http_parser::HTTP_PROPFIND => Some(PROPFIND),
    http_parser::HTTP_PROPPATCH => Some(PROPPATCH),
    http_parser::HTTP_SEARCH => Some(SEARCH),
    http_parser::HTTP_UNLOCK => Some(UNLOCK),
    http_parser::HTTP_REPORT => Some(REPORT),
    http_parser::HTTP_MKACTIVITY => Some(MKACTIVITY),
    http_parser::HTTP_CHECKOUT => Some(CHECKOUT),
    http_parser::HTTP_MERGE => Some(MERGE),
    http_parser::HTTP_MSEARCH => Some(MSEARCH),
    http_parser::HTTP_NOTIFY => Some(NOTIFY),
    http_parser::HTTP_SUBSCRIBE => Some(SUBSCRIBE),
    http_parser::HTTP_UNSUBSCRIBE => Some(UNSUBSCRIBE),
    http_parser::HTTP_PATCH => Some(PATCH),
    http_parser::HTTP_PURGE => Some(PURGE),
    _ => None
  }
}

pub fn initial_parser() -> Parser {
  let mut p = http_parser::Struct_http_parser {
    _type_flags: 0,
    state: 0,
    header_state: 0,
    index: 0,
    nread: 0,
    content_length: 0,
    http_major: 0,
    http_minor: 0,
    status_code: 0,
    method: 0,
    http_errno_upgrade: 0,
    data: null()
  };

  unsafe { http_parser_init(&p, HTTP_REQUEST); }

  let result = ParseResult::new();
  Parser { parser: ~p, result: result, offset: 0 }
}

#[test]
fn parse_simple_GET() {
  let request = "GET /foo HTTP/1.1\n\n";
  let p = initial_parser();

  let r = p.parse(request);

  assert!(r.success());
  assert!(r.result.url == Some(~"/foo"));
  assert!(r.result.method == Some(GET));
}

#[test]
fn parse_simple_GET_in_multiple_chunks() {
  let chunk_1 = "GET /fo";
  let chunk_2 = "o HTTP/1.1\n\n";
  let p = initial_parser();
  let r = p.parse(chunk_1)
           .parse(chunk_2);

  assert!(r.result.url == Some(~"/foo"));
}

#[test]
fn parse_headers() {
  let request = "\
  GET /foo HTTP/1.1\n\
  Header-1: pants\n\
  Header-2: bar\n\
  \n\
  ";

  let p = initial_parser();
  let r = p.parse(request);

  assert!(r.result.header("Header-1") == Some(~"pants"));
  assert!(r.result.header("Header-2") == Some(~"bar"));
  assert!(r.result.header("Non-Header") == None);
}

#[test]
fn parse_headers_in_multiple_chunks() {
  let chunk_1 = "GET /foo HTTP/1.1\nHe";
  let chunk_2 = "ader-1: pan";
  let chunk_3 = "ts\nHead";
  let chunk_4 = "er-2:";
  let chunk_5 = " bar\n\n";

  let p = initial_parser();
  let r = p.parse(chunk_1)
           .parse(chunk_2)
           .parse(chunk_3)
           .parse(chunk_4)
           .parse(chunk_5);

  assert!(r.result.header("Header-1") == Some(~"pants"));
  assert!(r.result.header("Header-2") == Some(~"bar"));
  assert!(r.result.header("Non-Header") == None);
}

#[test]
fn parse_error() {
  let request = "YURT /foo HTTP/1.1\n\n";
  let p = initial_parser();
  let r = p.parse(request).finish();

  assert!(!r.success());
  assert!(!r.upgrade());
  assert!(~"HPE_INVALID_METHOD" == r.error_name());
  assert!(~"invalid HTTP method" == r.error_description());
}

#[test]
fn parse_upgrade() {
  let request = "\
  GET /demo HTTP/1.1\n\
  Upgrade: WebSocket\n\
  Connection: Upgrade\n\
  Host: example.com\n\
  Origin: http://example.com\n\
  WebSocket-Protocol: sample\n\n\
  start of non-http content\
  ";

  let p = initial_parser();
  let r = p.parse(request);
  let offset = r.offset;
  let content = request.slice(offset, request.len());

  assert!(r.success());
  assert!(r.upgrade());
  assert!(content == ~"start of non-http content");
}

#[test]
fn parser_c_struct_size() {
  let size = sys::size_of::<http_parser::Struct_http_parser>();
  assert!(size == 32, ~"Expeced size 32, but was " + size.to_str());
}

#[test]
fn test_method_const_mapping() {
  assert!(http_method_const_to_enum(http_parser::HTTP_DELETE) == Some(DELETE));
  assert!(http_method_const_to_enum(http_parser::HTTP_GET) == Some(GET));
  assert!(http_method_const_to_enum(http_parser::HTTP_HEAD) == Some(HEAD));
  assert!(http_method_const_to_enum(http_parser::HTTP_POST) == Some(POST));
  assert!(http_method_const_to_enum(http_parser::HTTP_PUT) == Some(PUT));
  assert!(http_method_const_to_enum(http_parser::HTTP_CONNECT) == Some(CONNECT));
  assert!(http_method_const_to_enum(http_parser::HTTP_OPTIONS) == Some(OPTIONS));
  assert!(http_method_const_to_enum(http_parser::HTTP_TRACE) == Some(TRACE));
  assert!(http_method_const_to_enum(http_parser::HTTP_COPY) == Some(COPY));
  assert!(http_method_const_to_enum(http_parser::HTTP_LOCK) == Some(LOCK));
  assert!(http_method_const_to_enum(http_parser::HTTP_MKCOL) == Some(MKCOL));
  assert!(http_method_const_to_enum(http_parser::HTTP_MOVE) == Some(MOVE));
  assert!(http_method_const_to_enum(http_parser::HTTP_PROPFIND) == Some(PROPFIND));
  assert!(http_method_const_to_enum(http_parser::HTTP_PROPPATCH) == Some(PROPPATCH));
  assert!(http_method_const_to_enum(http_parser::HTTP_SEARCH) == Some(SEARCH));
  assert!(http_method_const_to_enum(http_parser::HTTP_UNLOCK) == Some(UNLOCK));
  assert!(http_method_const_to_enum(http_parser::HTTP_REPORT) == Some(REPORT));
  assert!(http_method_const_to_enum(http_parser::HTTP_MKACTIVITY) == Some(MKACTIVITY));
  assert!(http_method_const_to_enum(http_parser::HTTP_CHECKOUT) == Some(CHECKOUT));
  assert!(http_method_const_to_enum(http_parser::HTTP_MERGE) == Some(MERGE));
  assert!(http_method_const_to_enum(http_parser::HTTP_MSEARCH) == Some(MSEARCH));
  assert!(http_method_const_to_enum(http_parser::HTTP_NOTIFY) == Some(NOTIFY));
  assert!(http_method_const_to_enum(http_parser::HTTP_SUBSCRIBE) == Some(SUBSCRIBE));
  assert!(http_method_const_to_enum(http_parser::HTTP_UNSUBSCRIBE) == Some(UNSUBSCRIBE));
  assert!(http_method_const_to_enum(http_parser::HTTP_PATCH) == Some(PATCH));
  assert!(http_method_const_to_enum(http_parser::HTTP_PURGE) == Some(PURGE));
  assert!(http_method_const_to_enum(-1) == None);
}

