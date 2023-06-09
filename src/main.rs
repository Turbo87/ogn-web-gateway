use std::env;
use std::net::{IpAddr, SocketAddr};

use actix::prelude::*;
use actix_cors::Cors;
use actix_files::NamedFile;
use actix_ogn::OGNActor;
use actix_web::middleware::Logger;
use actix_web::{http, web, App, HttpServer, Responder};
use anyhow::{anyhow, Context, Result};
use clap::{self, value_t, Arg};
use log::debug;
use r2d2_redis::RedisConnectionManager;

mod api;
mod gateway;
mod geo;
mod ogn;
mod ogn_ddb;
mod redis;
mod units;
mod ws_client;

use crate::gateway::Gateway;
use crate::ogn_ddb::OGNDevicesUpdater;
use crate::redis::RedisExecutor;

const REDIS_WORKERS: usize = 7;

#[actix_web::main]
async fn main() -> Result<()> {
    let logger = setup_logging();
    log::set_max_level(logger.filter());

    let _sentry = if let Ok(sentry_dsn) = env::var("OGN_SENTRY_DSN") {
        let sentrylog = sentry::integrations::log::SentryLogger::with_dest(logger);
        log::set_boxed_logger(Box::new(sentrylog)).unwrap();
        Some(sentry::init(sentry::ClientOptions {
            dsn: Some(sentry_dsn.parse()?),
            ..Default::default()
        }))
    } else {
        log::set_boxed_logger(Box::new(logger)).unwrap();
        None
    };

    let matches = clap::App::new("OGN Web Gateway")
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

    let listen_host = value_t!(matches.value_of("host"), IpAddr)?;
    let listen_port = value_t!(matches.value_of("port"), u16)?;

    let redis_url = env::var("OGN_REDIS_URL").context("OGN_REDIS_URL must be set")?;
    let redis_url = r2d2_redis::redis::parse_redis_url(&redis_url)
        .map_err(|_| anyhow!("OGN_REDIS_URL could not be parsed"))?;

    let redis_connection_manager = RedisConnectionManager::new(redis_url)?;
    let redis_pool = r2d2_redis::r2d2::Pool::builder().build(redis_connection_manager)?;

    let redis_executor_addr = SyncArbiter::start(REDIS_WORKERS, move || {
        RedisExecutor::new(redis_pool.clone())
    });

    let updater_redis_addr = redis_executor_addr.clone();
    let _ogn_device_updater_addr = OGNDevicesUpdater {
        redis: updater_redis_addr,
    }
    .start();

    // Start "gateway" actor in separate thread
    let gateway_redis_addr = redis_executor_addr.clone();
    let gateway: Addr<_> = Gateway::new(gateway_redis_addr).start();

    // Start OGN client in separate thread
    let gw = gateway.clone();
    let _ogn_addr: Addr<_> = Supervisor::start(|_| OGNActor::new(gw.recipient()));

    // Origin for WebSocket CORS validation, e.g. https://ogn.cloud
    let cors_origin = env::var("OGN_CORS_ORIGIN").
        context("OGN_CORS_ORIGIN must be set a URI")?;

    debug!("Listening on {}:{}", listen_host, listen_port);

    // Create Http server with websocket support
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&cors_origin)
            .allow_any_method()
            .allowed_headers(vec![http::header::ACCEPT, http::header::CONTENT_TYPE])
            .max_age(3600);
        App::new()
            .app_data(web::Data::new(gateway.clone()))
            .app_data(web::Data::new(redis_executor_addr.clone()))
            .wrap(Logger::default())
            .service(
                web::scope("/api")
                    .wrap(cors)
                    .route("/ddb", web::get().to(api::ddb::get))
                    .route("/status", web::get().to(api::status::get))
                    .route("/records/{id}", web::get().to(api::records::get))
                    .route("/live", web::get().to(api::live::get)),
            )
            .route("/", web::get().to(index))
    })
    .bind(SocketAddr::new(listen_host, listen_port))?
    .run()
    .await?;

    Ok(())
}

async fn index() -> impl Responder {
    NamedFile::open("static/websocket.html")
}

fn setup_logging() -> pretty_env_logger::env_logger::Logger {
    let mut log_builder = pretty_env_logger::formatted_builder();
    if let Ok(s) = env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    }
    log_builder.build()
}
