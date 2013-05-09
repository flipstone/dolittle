use std::*;
use core;
use parser::*;
use websockets::protocol;

pub fn run_main() {
  run_server();
}

fn handle_socket(socket: &net_tcp::TcpSocket) {
  let mut parser = initial_parser();
  let mut chunk = ~"";

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

  chunk = str::from_slice(chunk.slice(parser.offset, chunk.len()));

  let acceptance = accept_websocket(&parser);

  socket.write(acceptance.to_websocket_response_str().to_bytes());

  println(acceptance.to_websocket_response_str());

  if acceptance.is_ok() {
    handle_websocket(chunk, socket);
  }
}

fn accept_websocket(parser: &Parser) -> protocol::AcceptResult {
  protocol::accept_headers(&parser.result.headers)
}

fn handle_websocket(mut body_chunk: ~str,
                    socket: &net_tcp::TcpSocket) {

  loop {
    println(~"Echoing body: " + body_chunk);
    socket.write(body_chunk.to_bytes());

    match socket.read(0) {
      Ok(bytes) => body_chunk = str::from_bytes(bytes),
      _ => break
    }
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

