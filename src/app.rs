use actix::*;
use actix_web::{fs, http, ws, App, HttpResponse, HttpRequest, Responder, Json, AsyncResponder, Error};
use futures::future::Future;

use systemstat::{self, Platform};

use db::DbExecutor;
use gateway::{self, Gateway};
use ws_client::WSClient;

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
        .resource("/api/status", |r| r.method(http::Method::GET).with(status))
        // websocket
        .resource("/ws/", |r| r.route().f(|req| ws::start(req, WSClient::default())))
        // static resources
        .handler("/static/", fs::StaticFiles::new("static/").unwrap())
}

#[derive(Serialize)]
struct Status {
    load: Option<(f32, f32, f32)>,
    users: usize,
}

fn status(req: HttpRequest<AppState>) -> impl Responder {
    req.state().gateway.send(gateway::RequestStatus).from_err::<Error>()
        .and_then(|res| {
            let sys = systemstat::System::new();

            Ok(Json(Status {
                load: sys.load_average().ok().map(|load| (load.one, load.five, load.fifteen)),
                users: res.users,
            }))
        })
        .responder()
}
