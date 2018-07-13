use actix_web::{HttpRequest, Responder, Json, AsyncResponder, Error};
use futures::future::Future;

use systemstat::{self, Platform};

use ::app::AppState;
use ::db;
use ::gateway;

#[derive(Serialize)]
struct Status {
    load: Option<(f32, f32, f32)>,
    users: usize,
    positions: Option<i64>,
}

pub fn status(req: HttpRequest<AppState>) -> impl Responder {
    Future::join(
        req.state().gateway.send(gateway::RequestStatus).from_err::<Error>(),
        req.state().db.send(db::CountOGNPositions).from_err::<Error>()
    ).and_then(|(gateway_status, position_count)| {
        let sys = systemstat::System::new();

        Ok(Json(Status {
            load: sys.load_average().ok().map(|load| (load.one, load.five, load.fifteen)),
            users: gateway_status.users,
            positions: position_count,
        }))
    })
    .responder()
}
