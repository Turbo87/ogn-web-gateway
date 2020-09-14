use std::collections::HashMap;

use actix::prelude::*;
use actix_web::{error::ErrorInternalServerError, web, Responder};
use chrono::prelude::*;
use serde::Deserialize;

use crate::redis::{OGNPosition, ReadOGNPositions, RedisExecutor};

#[derive(Deserialize, Debug)]
pub struct GetQueryParams {
    before: Option<i64>,
    after: Option<i64>,
}

pub async fn get(
    (id, query, redis): (
        web::Path<String>,
        web::Query<GetQueryParams>,
        web::Data<Addr<RedisExecutor>>,
    ),
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

    let map = redis
        .send(ReadOGNPositions { ids, after, before })
        .await
        .map_err(ErrorInternalServerError)?
        .map_err(ErrorInternalServerError)?;

    Ok::<_, actix_web::Error>(web::Json(map.serialize()))
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
