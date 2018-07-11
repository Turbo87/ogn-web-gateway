use chrono;
use actix::prelude::*;
use rand::{self, Rng, ThreadRng};
use std::cell::RefCell;
use std::collections::HashMap;

use aprs;
use ws_client::{WSClient, SendText};
use actix_ogn::OGNMessage;
use time::time_to_datetime;

/// `Gateway` manages connected websocket clients and distributes
/// `OGNRecord` messages to them.
pub struct Gateway {
    sessions: HashMap<usize, Addr<WSClient>>,
    rng: RefCell<ThreadRng>,
}

impl Default for Gateway {
    fn default() -> Gateway {
        Gateway {
            sessions: HashMap::new(),
            rng: RefCell::new(rand::thread_rng()),
        }
    }
}

impl Actor for Gateway {
    type Context = Context<Self>;
}

/// New websocket client has connected.
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Addr<WSClient>,
}

impl Handler<Connect> for Gateway {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        debug!("Client connected ({} clients)", self.sessions.len());

        // register session with random id
        let id = self.rng.borrow_mut().gen::<usize>();
        self.sessions.insert(id, msg.addr);

        // send id back
        id
    }
}

/// Websocket client has disconnected.
#[derive(Message)]
pub struct Disconnect {
    pub id: usize,
}

impl Handler<Disconnect> for Gateway {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.id);

        debug!("Client disconnected ({} clients)", self.sessions.len());
    }
}

impl Handler<OGNMessage> for Gateway {
    type Result = ();

    fn handle(&mut self, message: OGNMessage, _: &mut Context<Self>) {
        if let Some(position) = aprs::parse(&message.raw) {
            let time = time_to_datetime(chrono::Utc::now().naive_utc(), position.time);

            let ws_message = format!(
                "{}|{}|{:.6}|{:.6}",
                position.id,
                time.timestamp(),
                position.longitude,
                position.latitude,
            );

            // log record to the console
            trace!("{}", ws_message);

            // distribute record to all connected websocket clients
            for addr in self.sessions.values() {
                addr.do_send(SendText(ws_message.clone()));
            }
        }
    }
}
