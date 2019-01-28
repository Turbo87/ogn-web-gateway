use actix::*;
use actix_web::middleware::cors::Cors;
use actix_web::middleware::Logger;
use actix_web::*;

use api;
use gateway::Gateway;
use redis::RedisExecutor;

pub struct AppState {
    pub redis: Addr<RedisExecutor>,
    pub gateway: Addr<Gateway>,
}

pub fn build_app(redis: Addr<RedisExecutor>, gateway: Addr<Gateway>) -> App<AppState> {
    App::with_state(AppState { redis, gateway })
        .middleware(Logger::default())
        .configure(|app| {
            Cors::for_app(app)
                .resource("/api/cors-proxy/{uri:.+}", |r| {
                    r.get().with(api::cors_proxy::get)
                })
                .resource("/api/ddb", |r| r.get().with(api::ddb::get))
                .resource("/api/status", |r| r.get().with(api::status::get))
                .resource("/api/records/{id}", |r| r.get().with(api::records::get))
                .resource("/api/live", |r| r.get().with(api::live::get))
                .register()
        })
        .route("/", http::Method::GET, |_: HttpRequest<_>| {
            fs::NamedFile::open("static/websocket.html")
        })
}
