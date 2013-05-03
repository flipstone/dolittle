use std::*;
use core;

pub fn run_main() {
  run_server();
}

fn run_server() {
  let port: uint = 12345;
  let ip = unsafe { net_ip::Ipv4(uv_ll::ip4_addr("0.0.0.0",port as int)) };
  let backlog = 10;
  let io_task = uv_global_loop::get();
  let on_establish: ~fn(core::comm::SharedChan<Option<net_tcp::TcpErrData>>) =
   |chan| { println("Listening"); };
  let new_connect: ~fn(net_tcp::TcpNewConnection,core::comm::SharedChan<Option<net_tcp::TcpErrData>>) =
   |conn, chan| {
     let (cont_po, cont_ch) = core::comm::stream::<option::Option<net_tcp::TcpErrData>>();

     do task::spawn {
       match net_tcp::accept(conn) {
         Ok(socket) => {
           cont_ch.send(None);
           let buf = net_tcp::socket_buf(socket);
           buf.write_line("Hello Socket");
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

