use actix_web::*;
use futures::future::Future;

use ::app::AppState;
use ::redis;

pub fn get((state, request): (State<AppState>, HttpRequest<AppState>)) -> impl Responder {
    state.redis.send(redis::ReadOGNDDB).from_err::<Error>()
        .and_then(move |result| result
            .map(|devices| {
                request
                    .build_response(http::StatusCode::OK)
                    .header("Access-Control-Allow-Origin", "*")
                    .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                    .header("Content-Type", "application/json")
                    .body(devices)
            })
            .map_err(|err| err.into())
        )
        .responder()
}
