use std::sha1;
use std::base64::ToBase64;
use headers::*;

fn accept_key(key: &str) -> ~str {
  let mut sha = sha1::sha1();

  sha.input_str(key);
  sha.input_str("258EAFA5-E914-47DA-95CA-C5AB0DC85B11");

  sha.result().to_base64()
}

#[deriving(Eq)]
struct WebsocketAcceptance {
  key_accept: ~str
}

#[deriving(Eq)]
enum WebsocketAcceptError {
  HOST_REQUIRED,
  WEBSOCKET_VERSION_REQUIRED,
  INVALID_WEBSOCKET_VERSION,
  WEBSOCKET_KEY_REQUIRED,
  UPGRADE_REQUIRED,
  UPGRADE_WEBSOCKET_REQUIRED,
  CONNECTION_REQUIRED,
  CONNECTION_UPGRADE_REQUIRED,
}

pub type AcceptResult = Result<WebsocketAcceptance,WebsocketAcceptError>;

pub fn accept_headers<T: Headers>(headers: &T) -> AcceptResult {

  if !headers.has_header("Host") {
    return Err(HOST_REQUIRED)
  }

  if !headers.has_header("Connection") {
    return Err(CONNECTION_REQUIRED)
  }

  if !headers.has_header_keyword("Connection", "Upgrade") {
    return Err(CONNECTION_UPGRADE_REQUIRED)
  }

  if !headers.has_header("Upgrade") {
    return Err(UPGRADE_REQUIRED)
  }

  if !headers.has_header_keyword("Upgrade", "websocket") {
    return Err(UPGRADE_WEBSOCKET_REQUIRED)
  }

  if !headers.has_header("Sec-WebSocket-Version") {
    return Err(WEBSOCKET_VERSION_REQUIRED)
  }

  if !headers.has_header_value("Sec-WebSocket-Version", "13") {
    return Err(INVALID_WEBSOCKET_VERSION)
  }

  let key = headers.get_header("Sec-WebSocket-Key");

  match key {
    Some(str) => {
      Ok(WebsocketAcceptance { key_accept: accept_key(str) })
    }

    _ => {
      Err(WEBSOCKET_KEY_REQUIRED)
    }
  }
}

impl AcceptResult {
  fn to_websocket_response_str(&self) -> ~str {
    if self.is_ok() {
      let acceptance = self.get_ref();
      ~"HTTP/1.1 101 Switching Protocols\r\n\
        Upgrade: websocket\r\n\
        Connection: Upgrade\r\n\
        Sec-WebSocket-Accept: " + acceptance.key_accept + "\r\n\r\n"
    } else {
      ~"HTTP/1.1 400 Bad Request\r\n\r\n"
    }
  }
}

#[test]
fn websocket_accept_key() {
  let key = ~"dGhlIHNhbXBsZSBub25jZQ==";
  let accept = ~"s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";

  assert!(accept_key(key) == accept);
}

#[test]
fn accept_headers_base_success() {
  let headers = acceptable_websocket_headers("dGhlIHNhbXBsZSBub25jZQ==");

  assert!(accept_headers(&headers) == Ok(WebsocketAcceptance{
    key_accept: ~"s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
  }));
}


#[test]
fn accept_headers_missing_host() {
  let mut headers = acceptable_websocket_headers("dGhlIHNhbXBsZSBub25jZQ==");
  headers.remove_header("Host");
  assert!(accept_headers(&headers) == Err(HOST_REQUIRED));
}

#[test]
fn accept_headers_missing_websocket_version() {
  let mut headers = acceptable_websocket_headers("dGhlIHNhbXBsZSBub25jZQ==");
  headers.remove_header("Sec-WebSocket-Version");
  assert!(accept_headers(&headers) == Err(WEBSOCKET_VERSION_REQUIRED));
}

#[test]
fn accept_headers_incorrect_websocket_version() {
  let mut headers = acceptable_websocket_headers("dGhlIHNhbXBsZSBub25jZQ==");
  headers.set_header("Sec-WebSocket-Version", "12");
  assert!(accept_headers(&headers) == Err(INVALID_WEBSOCKET_VERSION));
}

#[test]
fn accept_headers_missing_upgrade() {
  let mut headers = acceptable_websocket_headers("dGhlIHNhbXBsZSBub25jZQ==");
  headers.remove_header("Upgrade");
  assert!(accept_headers(&headers) == Err(UPGRADE_REQUIRED));
}

#[test]
fn accept_headers_invalid_upgrade() {
  let mut headers = acceptable_websocket_headers("dGhlIHNhbXBsZSBub25jZQ==");
  headers.set_header("Upgrade", "tls");
  assert!(accept_headers(&headers) == Err(UPGRADE_WEBSOCKET_REQUIRED));
}

#[test]
fn accept_headers_missing_connection() {
  let mut headers = acceptable_websocket_headers("dGhlIHNhbXBsZSBub25jZQ==");
  headers.remove_header("Connection");
  assert!(accept_headers(&headers) == Err(CONNECTION_REQUIRED));
}

#[test]
fn accept_headers_invalid_connection() {
  let mut headers = acceptable_websocket_headers("dGhlIHNhbXBsZSBub25jZQ==");
  headers.set_header("Connection", "keep-alive");
  assert!(accept_headers(&headers) == Err(CONNECTION_UPGRADE_REQUIRED));
}

#[test]
fn accept_headers_missing_key() {
  let mut headers = acceptable_websocket_headers("dGhlIHNhbXBsZSBub25jZQ==");
  headers.remove_header("Sec-WebSocket-Key");
  assert!(accept_headers(&headers) == Err(WEBSOCKET_KEY_REQUIRED));
}

#[test]
fn ok_accept_response_string() {
  let success: AcceptResult = Ok(WebsocketAcceptance {
    key_accept: ~"foobarbazbat"
  });

  let expected = "\
  HTTP/1.1 101 Switching Protocols\r\n\
  Upgrade: websocket\r\n\
  Connection: Upgrade\r\n\
  Sec-WebSocket-Accept: foobarbazbat\r\n\
  \r\n\
  ";

  assert!(expected == success.to_websocket_response_str())
}

#[test]
fn err_accept_response_string() {
  let success: AcceptResult = Err(UPGRADE_REQUIRED);

  let expected = "HTTP/1.1 400 Bad Request\r\n\r\n";
  assert!(expected == success.to_websocket_response_str())
}

fn acceptable_websocket_headers(key: &str) -> HeaderMap {
  let mut headers = HeaderMap::new();

  headers.set_header("Host", "example.com");
  headers.set_header("Origin", "example.com");
  headers.set_header("Upgrade", "websocket, websocket/2.0");
  headers.set_header("Connection", "Upgrade, Keep-Alive");
  headers.set_header("Sec-WebSocket-Key", key);
  headers.set_header("Sec-WebSocket-Version", "13");

  headers
}

