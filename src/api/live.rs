use actix::prelude::*;
use actix_web::{web, HttpRequest, Responder};
use actix_web_actors::ws;

use crate::gateway::Gateway;
use crate::ws_client::WSClient;

pub async fn get(
    req: HttpRequest,
    stream: web::Payload,
    gateway_addr: web::Data<Addr<Gateway>>,
) -> impl Responder {
    let gateway = gateway_addr.into_inner();
    ws::start(WSClient::new(gateway), &req, stream)
}
