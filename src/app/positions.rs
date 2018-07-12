use actix_web::{HttpRequest, Responder, Json, Error, AsyncResponder};
use futures::Future;

use ::app::AppState;
use ::db;

pub fn positions(req: HttpRequest<AppState>) -> impl Responder {
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
