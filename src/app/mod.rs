use actix::*;
use actix_web::{fs, http, ws, App, HttpResponse, };

use db::DbExecutor;
use gateway::Gateway;
use ws_client::WSClient;

mod positions;
mod status;

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
        .resource("/api/status", |r| r.method(http::Method::GET).with(status::status))
        .route("/api/{id}/positions", http::Method::GET, positions::positions)
        // websocket
        .resource("/ws/", |r| r.route().f(|req| ws::start(req, WSClient::default())))
        // static resources
        .handler("/static/", fs::StaticFiles::new("static/").unwrap())
}
