use actix::*;
use actix_web::{fs, http, ws, App, HttpResponse, HttpRequest, Responder, Json, AsyncResponder, Error};
use futures::future::Future;

use db::{self, DbExecutor};
use gateway::Gateway;
use ws_client::WSClient;

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
        .route("/api/{id}/positions", http::Method::GET, positions)
        // websocket
        .resource("/ws/", |r| r.route().f(|req| ws::start(req, WSClient::default())))
        // static resources
        .handler("/static/", fs::StaticFiles::new("static/").unwrap())
}

fn positions(req: HttpRequest<AppState>) -> impl Responder {
    let ogn_id = req.match_info().get("id").unwrap().to_owned();

    req.state().db.send(db::ReadOGNPositions { ogn_id }).from_err::<Error>()
        .and_then(|res: Option<Vec<db::models::OGNPosition>>| {
            let ogn_id_positions: Vec<String> = res.unwrap_or_else(|| Vec::new()).iter()
                .map(|pos| format!(
                    "{}|{:.6}|{:.6}",
                    pos.time.timestamp(),
                    pos.longitude,
                    pos.latitude,
                ))
                .collect();

            Ok(Json(ogn_id_positions))
        })
        .responder()
}
