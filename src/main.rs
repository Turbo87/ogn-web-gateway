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

mod ogn_client;
mod gateway;
mod ws_client;

use ogn_client::OGNClient;

fn main() {
    let sys = actix::System::new("ogn-ws-gateway");

    // Start "gateway" actor in separate thread
    let gateway: Addr<Syn, _> = Arbiter::start(|_| gateway::Gateway::default());

    // Start OGN client in separate thread
    // TODO: Restart when connection drops
    let gw = gateway.clone();
    Arbiter::start(|_| OGNClient::new(gw.recipient()));

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
