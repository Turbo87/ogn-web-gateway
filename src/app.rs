use actix::*;
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
        .route("/", http::Method::GET, |_: HttpRequest<_>| {
            fs::NamedFile::open("static/websocket.html")
        })
        .route(
            "/api/cors-proxy/{uri:.+}",
            http::Method::GET,
            api::cors_proxy::get,
        )
        .route("/api/ddb", http::Method::GET, api::ddb::get)
        .route("/api/status", http::Method::GET, api::status::get)
        .route("/api/records/{id}", http::Method::GET, api::records::get)
        .route("/api/live", http::Method::GET, api::live::get)
}
