use actix_web::*;
use futures::{Future, Stream};

pub fn get(uri: Path<String>) -> impl Responder {
    client::ClientRequest::get(uri.as_ref())
        .finish()
        .unwrap()
        .send()
        .map_err(Error::from)
        .and_then(|resp| {
            let status = resp.status();

            Ok(HttpResponse::build(status)
                // read one chunk from client response and send this chunk to a server response
                // .from_err() converts PayloadError to an Error
                .body(Body::Streaming(Box::new(resp.payload().from_err()))))
        })
        .responder()
}
