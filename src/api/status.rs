use actix::prelude::*;
use actix_web::{error::ErrorInternalServerError, web, Responder};
use serde::Serialize;

use systemstat::{self, Platform};

use crate::gateway;

#[derive(Serialize)]
struct Status {
    load: Option<(f32, f32, f32)>,
    users: usize,
    positions: Option<u64>,
}

pub async fn get(gateway: web::Data<Addr<gateway::Gateway>>) -> impl Responder {
    let gateway_status = gateway
        .send(gateway::RequestStatus)
        .await
        .map_err(ErrorInternalServerError)?;

    let sys = systemstat::System::new();

    let load = sys
        .load_average()
        .ok()
        .map(|load| (load.one, load.five, load.fifteen));

    Ok::<_, actix_web::Error>(web::Json(Status {
        load,
        users: gateway_status.users,
        positions: gateway_status.record_count,
    }))
}
