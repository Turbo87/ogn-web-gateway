extern crate pretty_env_logger;
#[macro_use] extern crate log;

extern crate rand;

#[macro_use]
extern crate actix;
extern crate actix_web;
extern crate actix_ogn;
extern crate futures;

extern crate chrono;
extern crate regex;
#[macro_use] extern crate lazy_static;

extern crate sentry;

extern crate serde;
#[macro_use] extern crate serde_derive;

extern crate systemstat;

use actix::*;
use actix_web::server::HttpServer;
use actix_web::{fs, http, ws, App, HttpResponse, HttpRequest, Responder, Json, AsyncResponder, Error};
use actix_ogn::OGNActor;
use futures::future::Future;

use std::env;

use systemstat::Platform;

mod aprs;
mod gateway;
mod time;
mod ws_client;

use gateway::Gateway;

pub struct AppState {
    gateway: Addr<Gateway>,
}

fn main() {
    // reads sentry DSN from `SENTRY_DSN` environment variable
    let _sentry = sentry::init(());
    sentry::integrations::panic::register_panic_handler();

    setup_logging();

    let sys = actix::System::new("ogn-web-gateway");

    // Start "gateway" actor in separate thread
    let gateway: Addr<_> = Arbiter::start(|_| Gateway::new());

    // Start OGN client in separate thread
    let gw = gateway.clone();
    let _ogn_addr: Addr<_> = Supervisor::start(|_| OGNActor::new(gw.recipient()));

    // Create Http server with websocket support
    HttpServer::new(move || {
        let state = AppState {
            gateway: gateway.clone(),
        };

        App::with_state(state)
            // redirect to websocket.html
            .resource("/", |r| r.method(http::Method::GET).f(|_| {
                HttpResponse::Found()
                    .header("LOCATION", "/static/websocket.html")
                    .finish()
            }))
            .resource("/api/status", |r| r.method(http::Method::GET).with(status))
            // websocket
            .resource("/ws/", |r| r.route().f(|req| ws::start(req, ws_client::WSClient::default())))
            // static resources
            .handler("/static/", fs::StaticFiles::new("static/").unwrap())
    }).bind("127.0.0.1:8080")
        .unwrap()
        .start();

    info!("Started http server: 127.0.0.1:8080");

    sys.run();
}

fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_builder().unwrap();
    if let Ok(s) = env::var("RUST_LOG") {
        log_builder.parse(&s);
    }
    let logger = log_builder.build();
    let options = sentry::integrations::log::LoggerOptions {
        global_filter: Some(logger.filter()),
        ..Default::default()
    };
    sentry::integrations::log::init(Some(Box::new(logger)), options);
}

#[derive(Serialize)]
struct Status {
    load: Option<(f32, f32, f32)>,
    users: usize,
}

fn status(req: HttpRequest<AppState>) -> impl Responder {
    req.state().gateway.send(gateway::RequestStatus).from_err::<Error>()
        .and_then(|res| {
            let sys = systemstat::System::new();

            Ok(Json(Status {
                load: sys.load_average().ok().map(|load| (load.one, load.five, load.fifteen)),
                users: res.users,
            }))
        })
        .responder()
}
