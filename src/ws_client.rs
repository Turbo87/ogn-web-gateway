use std::time::Duration;

use actix::prelude::*;
use actix_web_actors::ws;

use crate::gateway;
use crate::geo::BoundingBox;

pub struct WSClient {
    buffer: String,
    gateway: Addr<gateway::Gateway>,
}

impl WSClient {
    pub fn new(gateway: Addr<gateway::Gateway>) -> WSClient {
        WSClient {
            buffer: String::new(),
            gateway,
        }
    }

    pub fn handle_message(&mut self, text: &str, ctx: &mut <Self as Actor>::Context) {
        if text.starts_with("+id|") {
            self.gateway.do_send(gateway::SubscribeToId {
                id: text[4..].to_owned(),
                addr: ctx.address(),
            });
        } else if text.starts_with("-id|") {
            self.gateway.do_send(gateway::UnsubscribeFromId {
                id: text[4..].to_owned(),
                addr: ctx.address(),
            });
        } else if text.starts_with("bbox|") {
            if let Some(bbox) = BoundingBox::try_parse(&text[5..]) {
                self.gateway.do_send(gateway::SetBoundingBox {
                    addr: ctx.address(),
                    bbox,
                });
            }
        }
    }

    pub fn flush(&mut self, ctx: &mut <Self as Actor>::Context) {
        if !self.buffer.is_empty() {
            ctx.text(&self.buffer);
            self.buffer.clear();
        }
    }
}

impl Actor for WSClient {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // register self in gateway.
        let addr: Addr<_> = ctx.address();
        self.gateway.do_send(gateway::Connect { addr });

        ctx.run_interval(Duration::from_millis(250), |act, ctx| {
            act.flush(ctx);
        });
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        // notify gateway
        let addr: Addr<_> = ctx.address();
        self.gateway.do_send(gateway::Disconnect { addr });

        Running::Stop
    }
}

#[derive(Message, Clone)]
pub struct SendText(pub String);

impl Handler<SendText> for WSClient {
    type Result = ();

    fn handle(&mut self, message: SendText, _ctx: &mut Self::Context) {
        if !self.buffer.is_empty() {
            self.buffer += "\n";
        }

        self.buffer += &message.0;
    }
}

/// WebSocket message handler
impl StreamHandler<ws::Message, ws::ProtocolError> for WSClient {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Ping(msg) => ctx.pong(&msg),
            ws::Message::Close(_) => ctx.stop(),
            ws::Message::Text(text) => self.handle_message(&text, ctx),
            _ => {}
        }
    }
}
