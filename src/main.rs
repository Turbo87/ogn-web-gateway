extern crate pretty_env_logger;
#[macro_use] extern crate log;

extern crate failure;

extern crate rand;

#[macro_use]
extern crate actix;
extern crate actix_web;
extern crate actix_ogn;
extern crate futures;

extern crate r2d2;
extern crate r2d2_redis;
extern crate redis as _redis;
#[macro_use] extern crate diesel;
extern crate chrono;
extern crate regex;
#[macro_use] extern crate lazy_static;

extern crate sentry;

extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate bincode;

extern crate systemstat;

#[cfg(test)] #[macro_use] extern crate approx;

use actix::*;
use actix_web::server::HttpServer;
use actix_ogn::OGNActor;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use r2d2_redis::RedisConnectionManager;

use std::env;

mod api;
mod app;
mod aprs;
mod db;
mod gateway;
mod geo;
mod ogn_ddb;
mod redis;
mod time;
mod units;
mod ws_client;

use app::build_app;
use db::DbExecutor;
use gateway::Gateway;
use ogn_ddb::OGNDevicesUpdater;
use redis::RedisExecutor;

const DB_WORKERS: usize = 7;
const REDIS_WORKERS: usize = 7;

fn main() {
    // reads sentry DSN from `SENTRY_DSN` environment variable
    let _sentry = sentry::init(());
    sentry::integrations::panic::register_panic_handler();

    setup_logging();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
    let redis_url = _redis::parse_redis_url(&redis_url).unwrap();

    let sys = actix::System::new("ogn-web-gateway");

    let db_connection_manager = ConnectionManager::<PgConnection>::new(database_url);
    let db_pool = Pool::builder()
        .max_size(DB_WORKERS as u32)
        .build(db_connection_manager)
        .expect("Failed to create pool.");

    let db_executor_addr = SyncArbiter::start(DB_WORKERS, move || {
        DbExecutor::new(db_pool.clone())
    });


    let redis_connection_manager = RedisConnectionManager::new(redis_url).unwrap();
    let redis_pool = r2d2::Pool::builder()
        .build(redis_connection_manager)
        .unwrap();

    let redis_executor_addr = SyncArbiter::start(REDIS_WORKERS, move || {
        RedisExecutor::new(redis_pool.clone())
    });


    let updater_db_addr = db_executor_addr.clone();
    let _ogn_device_updater_addr = Arbiter::start(|_| OGNDevicesUpdater { db: updater_db_addr });

    // Start "gateway" actor in separate thread
    let gateway_db_addr = db_executor_addr.clone();
    let gateway_redis_addr = redis_executor_addr.clone();
    let gateway: Addr<_> = Arbiter::start(|_| Gateway::new(gateway_db_addr, gateway_redis_addr));

    // Start OGN client in separate thread
    let gw = gateway.clone();
    let _ogn_addr: Addr<_> = Supervisor::start(|_| OGNActor::new(gw.recipient()));

    // Create Http server with websocket support
    HttpServer::new(move || build_app(db_executor_addr.clone(), redis_executor_addr.clone(), gateway.clone()))
        .bind("127.0.0.1:8080")
        .unwrap()
        .start();

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