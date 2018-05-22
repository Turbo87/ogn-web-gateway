extern crate futures;
extern crate rand;
extern crate tokio_core;
extern crate tokio_io;
extern crate regex;
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate actix;
extern crate actix_web;

use actix::*;
use actix_web::server::HttpServer;
use actix_web::{fs, http, ws, App, HttpResponse};

use std::str::FromStr;
use tokio_core::net::TcpStream;
use tokio_io::codec::{FramedRead, LinesCodec};
use futures::Future;
use std::{net, process};
use tokio_io::AsyncRead;

mod ogn_client;
mod gateway;
mod ws_client;

use ogn_client::OGNClient;

fn main() {
    let sys = actix::System::new("ogn-ws-gateway");

    // Start "gateway" actor in separate thread
    let gateway: Addr<Syn, _> = Arbiter::start(|_| gateway::Gateway::default());

    // Start OGN client in separate thread
    // TODO: Move connect() into actor?
    // TODO: Restart when connection drops
    // TODO: Resolve domain via DNS instead of hardcoding IP
    let gw = gateway.clone();
    let addr = net::SocketAddr::from_str("37.187.40.234:10152").unwrap();
    Arbiter::handle().spawn(
        TcpStream::connect(&addr, Arbiter::handle())
            .and_then(|stream| {
                let _addr: Addr<Syn, _> = OGNClient::create(|ctx| {
                    let (r, w) = stream.split();
                    OGNClient::add_stream(FramedRead::new(r, LinesCodec::new()), ctx);
                    OGNClient::new(gw.recipient(), actix::io::FramedWrite::new(w,LinesCodec::new(), ctx))
                });

                futures::future::ok(())
            })
            .map_err(|e| {
                println!("Can not connect to server: {}", e);
                process::exit(1)
            }),
    );

    // Create Http server with websocket support
    HttpServer::new(move || {
        // Websocket sessions state
        let state = ws_client::WSClientState::new(gateway.clone());

        App::with_state(state)
            // redirect to websocket.html
            .resource("/", |r| r.method(http::Method::GET).f(|_| {
                HttpResponse::Found()
                    .header("LOCATION", "/static/websocket.html")
                    .finish()
            }))
            // websocket
            .resource("/ws/", |r| r.route().f(|req| ws::start(req, ws_client::WSClient::default())))
            // static resources
            .handler("/static/", fs::StaticFiles::new("static/"))
    }).bind("127.0.0.1:8080")
        .unwrap()
        .start();

    println!("Started http server: 127.0.0.1:8080");

    sys.run();
}
