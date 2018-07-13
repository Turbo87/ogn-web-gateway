use actix::*;
use actix_web::{fs, http, App, HttpResponse};

use db::DbExecutor;
use gateway::Gateway;
use ::api;

pub struct AppState {
    pub db: Addr<DbExecutor>,
    pub gateway: Addr<Gateway>,
}

pub fn build_app(db: Addr<DbExecutor>, gateway: Addr<Gateway>) -> App<AppState> {
    App::with_state(AppState { db, gateway })
        // redirect to websocket.html
        .resource("/", |r| r.method(http::Method::GET).f(|_| {
            HttpResponse::Found()
                .header("LOCATION", "/static/websocket.html")
                .finish()
        }))
        .route("/api/status", http::Method::GET,  api::status::get)
        .route("/api/{id}/positions", http::Method::GET, api::positions::get)
        .route("/api/live", http::Method::GET, api::live::get)
        // static resources
        .handler("/static/", fs::StaticFiles::new("static/").unwrap())
}
