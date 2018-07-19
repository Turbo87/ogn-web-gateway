use actix::*;
use actix_web::*;
use actix_web::middleware::Logger;

use db::DbExecutor;
use gateway::Gateway;
use ::api;

pub struct AppState {
    pub db: Addr<DbExecutor>,
    pub gateway: Addr<Gateway>,
}

pub fn build_app(db: Addr<DbExecutor>, gateway: Addr<Gateway>) -> App<AppState> {
    App::with_state(AppState { db, gateway })
        .middleware(Logger::default())
        .route("/", http::Method::GET, |_: HttpRequest<_>| {
            fs::NamedFile::open("static/websocket.html")
        })
        .route("/api/ddb", http::Method::GET,  api::ddb::get)
        .route("/api/status", http::Method::GET,  api::status::get)
        .route("/api/records/{id}", http::Method::GET, api::records::get)
        .route("/api/live", http::Method::GET, api::live::get)
}
