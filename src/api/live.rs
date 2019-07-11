use actix_web::{ws, HttpRequest, Responder};

use crate::app::AppState;
use crate::ws_client::WSClient;

pub fn get(req: HttpRequest<AppState>) -> impl Responder {
    ws::start(&req, WSClient::default())
}
