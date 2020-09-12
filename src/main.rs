use std::env;
use std::net::{IpAddr, SocketAddr};

use ::actix::prelude::*;
use ::actix_cors::Cors;
use ::actix_files::NamedFile;
use ::actix_ogn::OGNActor;
use ::actix_web::middleware::Logger;
use ::actix_web::{web, App, HttpServer};
use ::anyhow::{anyhow, Context, Result};
use ::clap::{self, value_t, Arg};
use ::log::debug;
use ::r2d2_redis::RedisConnectionManager;

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

fn main() -> Result<()> {
    let logger = setup_logging();

    if let Ok(sentry_dsn) = env::var("SENTRY_DSN") {
        let log_integration =
            sentry::integrations::log::LogIntegration::default().with_env_logger_dest(Some(logger));

        let sentry_options = sentry::ClientOptions {
            // reads sentry DSN from `SENTRY_DSN` environment variable
            dsn: Some(sentry_dsn.parse()?),
            ..Default::default()
        }
        .add_integration(log_integration);

        let _sentry = sentry::init(sentry_options);
    } else {
        let max_level = logger.filter();
        let r = log::set_boxed_logger(Box::new(logger));
        if r.is_ok() {
            log::set_max_level(max_level);
        }
    }

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

    let redis_url = env::var("REDIS_URL").context("REDIS_URL must be set")?;
    let redis_url = r2d2_redis::redis::parse_redis_url(&redis_url)
        .map_err(|_| anyhow!("REDIS_URL could not be parsed"))?;

    let sys = actix::System::new("ogn-web-gateway");

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

    debug!("Listening on {}:{}", listen_host, listen_port);

    // Create Http server with websocket support
    HttpServer::new(move || {
        App::new()
            .data(gateway.clone())
            .data(redis_executor_addr.clone())
            .wrap(Logger::default())
            .service(
                web::scope("/api")
                    .wrap(Cors::default())
                    .route("/ddb", web::to_async(api::ddb::get))
                    .route("/status", web::to_async(api::status::get))
                    .route("/records/{id}", web::to_async(api::records::get))
                    .route("/live", web::to(api::live::get)),
            )
            .route("/", web::to(|| NamedFile::open("static/websocket.html")))
    })
    .bind(SocketAddr::new(listen_host, listen_port))?
    .start();

    sys.run()?;

    Ok(())
}

fn setup_logging() -> pretty_env_logger::env_logger::Logger {
    let mut log_builder = pretty_env_logger::formatted_builder();
    if let Ok(s) = env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    }
    log_builder.build()
}
