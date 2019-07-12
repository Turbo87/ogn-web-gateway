use actix::prelude::*;
use actix_web::{web, Error, Responder};
use futures::future::Future;
use serde::Serialize;

use systemstat::{self, Platform};

use crate::gateway;

#[derive(Serialize)]
struct Status {
    load: Option<(f32, f32, f32)>,
    users: usize,
    positions: Option<u64>,
}

pub fn get(
    gateway: web::Data<Addr<gateway::Gateway>>,
) -> impl Future<Item = impl Responder, Error = Error> {
    gateway
        .send(gateway::RequestStatus)
        .from_err::<Error>()
        .and_then(|gateway_status| {
            let sys = systemstat::System::new();

            let load = sys
                .load_average()
                .ok()
                .map(|load| (load.one, load.five, load.fifteen));

            Ok(web::Json(Status {
                load,
                users: gateway_status.users,
                positions: gateway_status.record_count,
            }))
        })
}
