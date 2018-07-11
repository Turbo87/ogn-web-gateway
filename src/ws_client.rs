use actix::*;
use actix_web::ws;
use actix_ogn::OGNMessage;

use gateway;

pub struct WSClientState {
    gateway: Addr<Syn, gateway::Gateway>,
}

impl WSClientState {
    pub fn new(gateway: Addr<Syn, gateway::Gateway>) -> WSClientState {
        WSClientState { gateway }
    }
}

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
    type Context = ws::WebsocketContext<Self, WSClientState>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // register self in gateway.
        let addr: Addr<Syn, _> = ctx.address();
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

impl Handler<OGNMessage> for WSClient {
    type Result = ();

    fn handle(&mut self, message: OGNMessage, ctx: &mut Self::Context) {
        // Pass raw APRS record to the WebSocket
        ctx.text(message.raw);
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
