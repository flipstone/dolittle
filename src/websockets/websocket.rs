use http::parser::*;
use websockets::protocol::*;

struct WebSocket<T> {
  socket: T
}

trait Transport {
  fn read(&self) -> Result<~[u8],Error>;
  fn write(&self, bytes: ~[u8]);
}

fn accept_websocket<T: Transport>(transport: T)
   -> Result<WebSocket<T>,~str> {

  match read_and_parse_request(&transport) {
    Ok(parser) => handle_accept_result(transport, accept_request(&parser)),
    Err(message) => Err(message)
  }
}

fn read_and_parse_request<T: Transport>(transport: &T) -> Result<Parser,~str> {
  let mut parser = initial_parser();
  let mut chunk;

  loop {
    let read = transport.read();

    match read {
      Ok(bytes) => {
        chunk = str::from_bytes(bytes);
        parser = parser.parse(chunk);

        if parser.upgrade() {
          return Ok(parser)
        }
      }

      Err(error) => return Err(error)
    }
  }
}

fn handle_accept_result<T: Transport>(transport: T, accept_result: AcceptResult)
   -> Result<WebSocket<T>,~str> {
  transport.write(accept_result.to_websocket_response_str().to_bytes());

  if accept_result.is_ok() {
    Ok(WebSocket { socket: transport })
  } else {
    Err(~"Failed to accept")
  }
}

static sample_handshake:&'static str =
  "GET /chat HTTP/1.1\n\
   Host: server.example.com\n\
   Upgrade: websocket\n\
   Connection: Upgrade\n\
   Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\n\
   Origin: http://example.com\n\
   Sec-WebSocket-Version: 13\n\n\
   ";

#[test]
fn accept_connection_sends_response() {
  let (server_socket, client_socket) = fake_connection();
  client_socket.fake_write_chunked(sample_handshake.to_bytes());

  let result = accept_websocket(server_socket);

  assert!(result.is_ok());

  let response = client_socket.read().map(|bytes| { str::from_bytes(*bytes) });

  assert!(response ==
    Ok(~"HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\
         \r\n"))
}

#[test]
fn accept_connection_closes_on_handshake_error() {
  let (server_socket, client_socket) = fake_connection();
  client_socket.fake_write_chunked("GET /chat HTTP/1.1\n".to_bytes());
  client_socket.fake_error(~"OMG Test Error!");

  let result = accept_websocket(server_socket);
  assert!(!result.is_ok());
  assert!(result.get_err() == ~"OMG Test Error!");
  assert!(client_socket.fake_read() == Err(~"Attempt to read closed socket"));
}

#[test]
fn accept_connection_sends_response_an_closes_on_invalid_handshake() {
  let (server_socket, client_socket) = fake_connection();
  let bad_handshake =
    "GET /chat HTTP/1.1\n\
     Host: server.example.com\n\
     Upgrade: !!not-websockets!!\n\
     Connection: Upgrade\n\
     Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\n\
     Origin: http://example.com\n\
     Sec-WebSocket-Version: 13\n\n\
     ";
  client_socket.fake_write_chunked(bad_handshake.to_bytes());

  let result = accept_websocket(server_socket);

  assert!(!result.is_ok());

  let response = client_socket.read().map(|bytes| { str::from_bytes(*bytes) });

  assert!(response == Ok(~"HTTP/1.1 400 Bad Request\r\n\r\n"));
  assert!(client_socket.fake_read() == Err(~"Attempt to read closed socket"));
}

enum SocketState { OPEN, CLOSED }

enum FakePacket {
  Data(~[u8]), Error(~str), Close
}

struct FakeSocket {
  in: Port<FakePacket>,
  out: Chan<FakePacket>,
  state: @mut SocketState,
}

type Error = ~str;

fn fake_connection() -> (FakeSocket, FakeSocket) {
  let (stream_1_in, stream_2_out) = stream();
  let (stream_2_in, stream_1_out) = stream();
  let socket_1 = FakeSocket::open(stream_1_in, stream_1_out);
  let socket_2 = FakeSocket::open(stream_2_in, stream_2_out);

  (socket_1, socket_2)
}

impl FakeSocket {
  fn open(in: Port<FakePacket>, out: Chan<FakePacket>) -> FakeSocket {
    FakeSocket { in: in, out: out, state: @mut OPEN }
  }

  fn fake_close(&self) {
    self.out.send(Close);
    *self.state = CLOSED;
  }

  fn fake_error(&self, message: ~str) {
    self.out.send(Error(message));
  }

  fn fake_write(&self, bytes: &[u8]) {
    self.out.send(Data(bytes.to_owned()));
  }

  fn fake_write_chunked(&self, bytes: &[u8]) {
    for bytes.each |byte| {
      self.out.send(Data(~[*byte]));
    }
  }

  fn fake_read(&self) -> Result<~[u8],Error> {
    match *self.state {
      CLOSED => self.closed_read(),
      OPEN => {
        if self.in.peek() {
          self.do_read()
        } else {
          fail!(~"Tried to read bytes from fake socket, \
                  but no more were available.")
        }
      }
    }
  }

  fn do_read(&self) -> Result<~[u8],Error> {
    match self.in.recv() {
      Data(bytes) => Ok(bytes),
      Error(message) => Err(message),
      Close => {
         self.handle_close();
         self.closed_read()
      }
    }
  }

  fn handle_close(&self) {
    *self.state = CLOSED;
  }

  fn closed_read(&self) -> Result<~[u8],Error> {
    Err(~"Attempt to read closed socket")
  }
}

impl Transport for FakeSocket {
  fn read(&self) -> Result<~[u8],Error> {
    self.fake_read()
  }

  fn write(&self, bytes: ~[u8]) {
    self.fake_write(bytes)
  }
}

#[unsafe_destructor]
impl Drop for FakeSocket {
  fn finalize(&self) {
    self.out.send(Close);
  }
}
