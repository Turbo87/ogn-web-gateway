use actix::prelude::*;
use actix_web::{web, Error, Responder};
use futures::future::Future;
use serde::Serialize;

use systemstat::{self, Platform};

use crate::gateway;
use crate::redis;

#[derive(Serialize)]
struct Status {
    load: Option<(f32, f32, f32)>,
    users: usize,
    positions: u64,
}

pub fn get(
    gateway: web::Data<Addr<gateway::Gateway>>,
    redis: web::Data<Addr<redis::RedisExecutor>>,
) -> impl Future<Item = impl Responder, Error = Error> {
    Future::join(
        gateway.send(gateway::RequestStatus).from_err::<Error>(),
        redis.send(redis::CountOGNPositions).from_err::<Error>(),
    )
    .and_then(|(gateway_status, position_count)| {
        let sys = systemstat::System::new();

        position_count
            .map(|position_count| {
                let load = sys
                    .load_average()
                    .ok()
                    .map(|load| (load.one, load.five, load.fifteen));

                web::Json(Status {
                    load,
                    users: gateway_status.users,
                    positions: position_count,
                })
            })
            .map_err(|err| err.into())
    })
}
