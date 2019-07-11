use actix::prelude::*;
use actix_web::{web, HttpRequest, Responder};
use actix_web_actors::ws;

use crate::gateway::Gateway;
use crate::ws_client::WSClient;

pub fn get(
    req: HttpRequest,
    stream: web::Payload,
    gateway_addr: web::Data<Addr<Gateway>>,
) -> impl Responder {
    ws::start(WSClient::new((*gateway_addr).clone()), &req, stream)
}
