use std::*;
use core;
use http::parser::*;
use http::parser::Headers;
use websockets::framing::parser::*;
use websockets::framing::types::*;
use websockets::messaging::*;
use websockets::protocol;

pub fn run_main() {
  run_server();
}

fn handle_socket(socket: &net_tcp::TcpSocket) {
  let mut parser = initial_parser();
  let mut chunk;

  loop {
    let result = socket.read(0);

    match result {
      Ok(bytes) => {
        chunk = str::from_bytes(bytes);
        parser = parser.parse(chunk);
      }

      _ => {
        return;
      }
    }

    if parser.upgrade() {
      break;
    }
  }

  println(~"Protocol requested: " +
    sys::log_str(&parser.get_header("sec-websocket-protocol")));

  println(~"Extensions requested: " +
    sys::log_str(&parser.get_header("sec-websocket-extensions")));

  chunk = str::from_slice(chunk.slice(parser.offset, chunk.len()));

  let acceptance = accept_websocket(&parser);

  socket.write(acceptance.to_websocket_response_str().to_bytes());

  println(acceptance.to_websocket_response_str());

  if acceptance.is_ok() {
    handle_websocket(chunk, socket);
  }
}

fn accept_websocket(parser: &Parser) -> protocol::AcceptResult {
  protocol::accept_request(parser)
}

fn handle_websocket(body_chunk: ~str,
                    socket: &net_tcp::TcpSocket) {
  println(~"Handling: " + sys::log_str(&socket.get_peer_addr()));

  let mut bytes = body_chunk.to_bytes();
  let mut frame_parser = FrameParser::new();
  let mut receiver = Receiver::new();

  loop {
    println(~"Trying to parse " + bytes.len().to_str() + ~" bytes");
    let result = frame_parser.parse(bytes);
    frame_parser = result.parser;
    bytes = vec::from_slice(bytes.tailn(result.bytes_parsed));

    if result.is_done() {
      frame_parser = FrameParser::new();

      let recv_frame = result.make_frame_done();

      if recv_frame.op_code.is_control() {
        handle_control_frame(recv_frame, socket);
      } else {
        receiver = handle_data_frame(recv_frame, receiver);
      }
    }

    if bytes.len() <= 0 {
      match socket.read(0) {
        Ok(new_bytes) => bytes = new_bytes,
        _ => break
      }
    }
  }
}

fn handle_control_frame(recv_frame: Frame,
                        socket: &net_tcp::TcpSocket) {
  println("Got Control Frame");
  println(sys::log_str(&recv_frame.op_code));
  println("");

  match recv_frame.op_code {
    PING => {
      let pong = Frame {
        fin: true,
        reserved: false,
        op_code: PONG,
        masking_key: None,
        payload_data: PayloadData::new().mask(None),
      };

      socket.write(pong.compose());
    },
    _ => {}
  }
}

fn handle_data_frame(recv_frame: Frame, receiver: Receiver) -> Receiver {
  let data = recv_frame.unmasked_payload();

  match receiver.next_fragment(recv_frame) {
    Receiving(r_prime) => {
      println("Got Message Fragment");
      r_prime
    },

    Received(msg) => {
      println("Got Message");
      Receiver::new()
    },

    ReceptionError(error) => {
      println("Got Reception Error");
      println(sys::log_str(&error));
      println("");
      Receiver::new()
    },
  }
}

fn run_server() {
  let port: uint = 12345;
  let ip = unsafe { net_ip::Ipv4(uv_ll::ip4_addr("0.0.0.0",port as int)) };
  let backlog = 10;
  let io_task = uv_global_loop::get();
  let on_establish: ~fn(core::comm::SharedChan<Option<net_tcp::TcpErrData>>) =
   |chan| { println(~"Listening on " + port.to_str()); };
  let new_connect: ~fn(net_tcp::TcpNewConnection,core::comm::SharedChan<Option<net_tcp::TcpErrData>>) =
   |conn, chan| {
     let (cont_po, cont_ch) = core::comm::stream::<option::Option<net_tcp::TcpErrData>>();

     do task::spawn {
       match net_tcp::accept(conn) {
         Ok(socket) => {
           cont_ch.send(None);
           println("Handling Socket");
           handle_socket(&socket);
         }
         Err(error) => {
           cont_ch.send(Some(error));
           println("Error during accept");
         }
       }
     };

     match cont_po.recv() {
       Some(error) => println(error.err_name + ~": " + error.err_msg),
       None => ()
     }
   };

  net_tcp::listen(ip,
                  port,
                  backlog,
                  &io_task,
                  on_establish,
                  new_connect);
}

