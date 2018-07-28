use std::collections::HashMap;

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

    let ids: Vec<_> = id.split(",").map(|s| s.to_owned()).collect();
    let mut id_records_map: HashMap<String, Vec<String>> = ids.iter().map(|id| (id.to_owned(), Vec::new())).collect();

    let db_request = db::ReadOGNPositions { ids: ids.clone(), after, before };

    state.db.send(db_request).from_err::<Error>()
        .and_then(|res: Option<Vec<db::models::OGNPosition>>| {
            res.unwrap_or_else(|| Vec::new()).iter().for_each(|pos| {
                let list = id_records_map.get_mut(&pos.ogn_id).unwrap();

                list.push(format!(
                    "{}|{:.6}|{:.6}|{}",
                    pos.time.timestamp(),
                    pos.longitude,
                    pos.latitude,
                    pos.altitude,
                ));
            });

            Ok(Json(id_records_map))
        })
        .responder()
}
