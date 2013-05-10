use std::sha1;
use std::base64::ToBase64;
use http::headers::*;
use http::request::*;

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
  GET_METHOD_REQUIRED,
  HTTP_1_PT_1_REQUIRED,
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

pub fn accept_request<T: Headers+Request>(request: &T) -> AcceptResult {
  if !(request.http_version() == Some(HttpVersion(1,1))) {
    return Err(HTTP_1_PT_1_REQUIRED)
  }

  if !(request.method() == Some(GET)) {
    return Err(GET_METHOD_REQUIRED)
  }

  if !request.has_header("Host") {
    return Err(HOST_REQUIRED)
  }

  if !request.has_header("Connection") {
    return Err(CONNECTION_REQUIRED)
  }

  if !request.has_header_keyword("Connection", "Upgrade") {
    return Err(CONNECTION_UPGRADE_REQUIRED)
  }

  if !request.has_header("Upgrade") {
    return Err(UPGRADE_REQUIRED)
  }

  if !request.has_header_keyword("Upgrade", "websocket") {
    return Err(UPGRADE_WEBSOCKET_REQUIRED)
  }

  if !request.has_header("Sec-WebSocket-Version") {
    return Err(WEBSOCKET_VERSION_REQUIRED)
  }

  if !request.has_header_value("Sec-WebSocket-Version", "13") {
    return Err(INVALID_WEBSOCKET_VERSION)
  }

  let key = request.get_header("Sec-WebSocket-Key");

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
fn accept_request_base_success() {
  let request = acceptable_websocket_request("dGhlIHNhbXBsZSBub25jZQ==");

  assert!(accept_request(&request) == Ok(WebsocketAcceptance{
    key_accept: ~"s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
  }));
}

#[test]
fn accept_request_not_get() {
  let mut request = acceptable_websocket_request("dGhlIHNhbXBsZSBub25jZQ==");
  request.method = Some(POST);
  assert!(accept_request(&request) == Err(GET_METHOD_REQUIRED));
}

#[test]
fn accept_request_not_http_1_pt_1() {
  let mut request = acceptable_websocket_request("dGhlIHNhbXBsZSBub25jZQ==");
  request.http_version = Some(HttpVersion(1,0));
  assert!(accept_request(&request) == Err(HTTP_1_PT_1_REQUIRED));
}

#[test]
fn accept_request_missing_host() {
  let mut request = acceptable_websocket_request("dGhlIHNhbXBsZSBub25jZQ==");
  request.headers.remove_header("Host");
  assert!(accept_request(&request) == Err(HOST_REQUIRED));
}

#[test]
fn accept_request_missing_websocket_version() {
  let mut request = acceptable_websocket_request("dGhlIHNhbXBsZSBub25jZQ==");
  request.headers.remove_header("Sec-WebSocket-Version");
  assert!(accept_request(&request) == Err(WEBSOCKET_VERSION_REQUIRED));
}

#[test]
fn accept_request_incorrect_websocket_version() {
  let mut request = acceptable_websocket_request("dGhlIHNhbXBsZSBub25jZQ==");
  request.headers.set_header("Sec-WebSocket-Version", "12");
  assert!(accept_request(&request) == Err(INVALID_WEBSOCKET_VERSION));
}

#[test]
fn accept_request_missing_upgrade() {
  let mut request = acceptable_websocket_request("dGhlIHNhbXBsZSBub25jZQ==");
  request.headers.remove_header("Upgrade");
  assert!(accept_request(&request) == Err(UPGRADE_REQUIRED));
}

#[test]
fn accept_request_invalid_upgrade() {
  let mut request = acceptable_websocket_request("dGhlIHNhbXBsZSBub25jZQ==");
  request.headers.set_header("Upgrade", "tls");
  assert!(accept_request(&request) == Err(UPGRADE_WEBSOCKET_REQUIRED));
}

#[test]
fn accept_request_missing_connection() {
  let mut request = acceptable_websocket_request("dGhlIHNhbXBsZSBub25jZQ==");
  request.headers.remove_header("Connection");
  assert!(accept_request(&request) == Err(CONNECTION_REQUIRED));
}

#[test]
fn accept_request_invalid_connection() {
  let mut request = acceptable_websocket_request("dGhlIHNhbXBsZSBub25jZQ==");
  request.headers.set_header("Connection", "keep-alive");
  assert!(accept_request(&request) == Err(CONNECTION_UPGRADE_REQUIRED));
}

#[test]
fn accept_request_missing_key() {
  let mut request = acceptable_websocket_request("dGhlIHNhbXBsZSBub25jZQ==");
  request.headers.remove_header("Sec-WebSocket-Key");
  assert!(accept_request(&request) == Err(WEBSOCKET_KEY_REQUIRED));
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

struct TestRequest {
  http_version: Option<HttpVersion>,
  method: Option<Method>,
  headers: HeaderMap
}

impl Request for TestRequest {
  fn method(&self) -> Option<Method> {
    self.method
  }

  fn http_version(&self) -> Option<HttpVersion> {
    self.http_version
  }
}

impl Headers for TestRequest {
  fn get_header(&self, name: &str) -> Option<~str> {
    self.headers.get_header(name)
  }

  fn has_header(&self, name: &str) -> bool {
    self.headers.has_header(name)
  }
}

fn acceptable_websocket_request(key: &str) -> TestRequest {
  let mut req = TestRequest {
    http_version: Some(HttpVersion(1,1)),
    method: Some(GET),
    headers: HeaderMap::new()
  };

  req.headers.set_header("Host", "example.com");
  req.headers.set_header("Origin", "example.com");
  req.headers.set_header("Upgrade", "websocket, websocket/2.0");
  req.headers.set_header("Connection", "Upgrade, Keep-Alive");
  req.headers.set_header("Sec-WebSocket-Key", key);
  req.headers.set_header("Sec-WebSocket-Version", "13");

  req
}

