use actix_web::*;
use chrono::prelude::*;
use futures::Future;

use ::app::AppState;
use ::db;

#[derive(Deserialize, Debug)]
pub struct GetQueryParams {
    before: Option<i64>,
    after: Option<i64>,
}

pub fn get((id, query, state): (Path<String>, Query<GetQueryParams>, State<AppState>)) -> impl Responder {
    let after = query.after
        .and_then(|it| NaiveDateTime::from_timestamp_opt(it, 0))
        .map(|it| DateTime::from_utc(it, Utc));

    let before = query.before
        .and_then(|it| NaiveDateTime::from_timestamp_opt(it, 0))
        .map(|it| DateTime::from_utc(it, Utc));

    let db_request = db::ReadOGNPositions { ogn_id: id.to_owned(), after, before };

    state.db.send(db_request).from_err::<Error>()
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
