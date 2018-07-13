use actix_web::*;
use futures::Future;

use ::app::AppState;
use ::db;


pub fn get((id, state): (Path<String>, State<AppState>)) -> impl Responder {
    state.db.send(db::ReadOGNPositions { ogn_id: id.to_owned() }).from_err::<Error>()
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
