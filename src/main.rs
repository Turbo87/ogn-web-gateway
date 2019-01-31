extern crate pretty_env_logger;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

extern crate failure;

extern crate actix;
extern crate actix_ogn;
extern crate actix_web;
extern crate futures;

extern crate chrono;
extern crate r2d2_redis;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate itertools;

extern crate sentry;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate serde_json;

extern crate systemstat;

#[cfg(test)]
#[macro_use]
extern crate approx;

use actix::*;
use actix_ogn::OGNActor;
use actix_web::server::HttpServer;

use r2d2_redis::RedisConnectionManager;

use clap::{App, Arg};
use std::env;
use std::net::{IpAddr, SocketAddr};

mod api;
mod app;
mod gateway;
mod geo;
mod ogn;
mod ogn_ddb;
mod redis;
mod units;
mod ws_client;

use app::build_app;
use gateway::Gateway;
use ogn_ddb::OGNDevicesUpdater;
use redis::RedisExecutor;

const REDIS_WORKERS: usize = 7;

fn main() {
    // reads sentry DSN from `SENTRY_DSN` environment variable
    let _sentry = sentry::init(());
    sentry::integrations::panic::register_panic_handler();

    setup_logging();

    let matches = App::new("OGN Web Gateway")
        .arg(
            Arg::with_name("host")
                .short("h")
                .long("host")
                .default_value("127.0.0.1")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .default_value("8080")
                .takes_value(true),
        )
        .get_matches();

    let listen_host = value_t!(matches.value_of("host"), IpAddr).unwrap();
    let listen_port = value_t!(matches.value_of("port"), u16).unwrap();

    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
    let redis_url = r2d2_redis::redis::parse_redis_url(&redis_url).unwrap();

    let sys = actix::System::new("ogn-web-gateway");

    let redis_connection_manager = RedisConnectionManager::new(redis_url).unwrap();
    let redis_pool = r2d2_redis::r2d2::Pool::builder()
        .build(redis_connection_manager)
        .unwrap();

    let redis_executor_addr = SyncArbiter::start(REDIS_WORKERS, move || {
        RedisExecutor::new(redis_pool.clone())
    });

    let updater_redis_addr = redis_executor_addr.clone();
    let _ogn_device_updater_addr = Arbiter::start(|_| OGNDevicesUpdater {
        redis: updater_redis_addr,
    });

    // Start "gateway" actor in separate thread
    let gateway_redis_addr = redis_executor_addr.clone();
    let gateway: Addr<_> = Arbiter::start(|_| Gateway::new(gateway_redis_addr));

    // Start OGN client in separate thread
    let gw = gateway.clone();
    let _ogn_addr: Addr<_> = Supervisor::start(|_| OGNActor::new(gw.recipient()));

    debug!("Listening on {}:{}", listen_host, listen_port);

    // Create Http server with websocket support
    HttpServer::new(move || build_app(redis_executor_addr.clone(), gateway.clone()))
        .bind(SocketAddr::new(listen_host, listen_port))
        .unwrap()
        .start();

    sys.run();
}

fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_builder();
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
