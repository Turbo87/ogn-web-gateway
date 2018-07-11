use actix::*;
use actix_web::ws;

use gateway;
use AppState;

pub struct WSClient {
    /// unique session id
    id: usize,
}

impl Default for WSClient {
    fn default() -> WSClient {
        WSClient {
            id: 0,
        }
    }
}

impl Actor for WSClient {
    type Context = ws::WebsocketContext<Self, AppState>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // register self in gateway.
        let addr: Addr<_> = ctx.address();
        ctx.state().gateway
            .send(gateway::Connect { addr })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    // something is wrong with the gateway
                    _ => ctx.stop(),
                }
                fut::ok(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        // notify gateway
        ctx.state().gateway.do_send(gateway::Disconnect { id: self.id });
        Running::Stop
    }
}

#[derive(Message, Clone)]
pub struct SendText(pub String);

impl Handler<SendText> for WSClient {
    type Result = ();

    fn handle(&mut self, message: SendText, ctx: &mut Self::Context) {
        ctx.text(message.0);
    }
}

/// WebSocket message handler
impl StreamHandler<ws::Message, ws::ProtocolError> for WSClient {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        debug!("WEBSOCKET MESSAGE: {:?}", msg);

        match msg {
            ws::Message::Ping(msg) => ctx.pong(&msg),
            ws::Message::Close(_) => ctx.stop(),
            _ => {},
        }
    }
}
