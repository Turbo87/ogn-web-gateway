use actix_web::{ws, HttpRequest, Responder};

use ::app::AppState;
use ::ws_client::WSClient;

pub fn live(req: HttpRequest<AppState>) -> impl Responder {
    ws::start(&req, WSClient::default())
}
