use actix::prelude::*;
use actix_web::{web, Error, Responder};
use futures::future::Future;

use crate::redis;

pub fn get(
    redis: web::Data<Addr<redis::RedisExecutor>>,
) -> impl Future<Item = impl Responder, Error = Error> {
    redis
        .send(redis::ReadOGNDDB)
        .from_err::<Error>()
        .and_then(move |result| {
            result
                .map(|devices| {
                    devices
                        .with_header("Access-Control-Allow-Origin", "*")
                        .with_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                        .with_header("Content-Type", "application/json")
                })
                .map_err(|_err| From::from(()))
        })
}
