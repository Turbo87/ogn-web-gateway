use actix::*;
use actix_web::*;

use db::DbExecutor;
use gateway::Gateway;
use ::api;

pub struct AppState {
    pub db: Addr<DbExecutor>,
    pub gateway: Addr<Gateway>,
}

pub fn build_app(db: Addr<DbExecutor>, gateway: Addr<Gateway>) -> App<AppState> {
    App::with_state(AppState { db, gateway })
        .route("/", http::Method::GET, |_: HttpRequest<_>| {
            fs::NamedFile::open("static/websocket.html")
        })
        .route("/api/status", http::Method::GET,  api::status::get)
        .route("/api/records/{id}", http::Method::GET, api::records::get)
        .route("/api/live", http::Method::GET, api::live::get)
}
