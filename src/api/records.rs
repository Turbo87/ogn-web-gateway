use std::collections::HashMap;

use actix_web::*;
use chrono::prelude::*;
use failure;
use futures::Future;
use serde::Deserialize;

use crate::app::AppState;
use crate::redis::{OGNPosition, ReadOGNPositions};

#[derive(Deserialize, Debug)]
pub struct GetQueryParams {
    before: Option<i64>,
    after: Option<i64>,
}

pub fn get(
    (id, query, state): (Path<String>, Query<GetQueryParams>, State<AppState>),
) -> impl Responder {
    let after = query
        .after
        .and_then(|it| NaiveDateTime::from_timestamp_opt(it, 0))
        .map(|it| DateTime::from_utc(it, Utc));

    let before = query
        .before
        .and_then(|it| NaiveDateTime::from_timestamp_opt(it, 0))
        .map(|it| DateTime::from_utc(it, Utc));

    let ids: Vec<_> = id.split(',').map(|s| s.to_owned()).collect();

    state
        .redis
        .send(ReadOGNPositions { ids, after, before })
        .from_err::<Error>()
        .and_then(
            |result: Result<HashMap<String, Vec<OGNPosition>>, failure::Error>| {
                result
                    .map(|map| Json(map.serialize()))
                    .map_err(|err| err.into())
            },
        )
        .responder()
}

trait SerializeRecords {
    fn serialize(self) -> HashMap<String, Vec<String>>;
}

impl SerializeRecords for HashMap<String, Vec<OGNPosition>> {
    fn serialize(self) -> HashMap<String, Vec<String>> {
        self.into_iter()
            .map(|(id, records)| {
                let serialized = records
                    .into_iter()
                    .map(|record| {
                        format!(
                            "{}|{:.6}|{:.6}|{}",
                            record.time.timestamp(),
                            record.longitude,
                            record.latitude,
                            record.altitude,
                        )
                    })
                    .collect();

                (id, serialized)
            })
            .collect()
    }
}
