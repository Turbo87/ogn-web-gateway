use actix::prelude::*;
use rand::{self, Rng, ThreadRng};
use std::cell::RefCell;
use std::collections::HashMap;

use ws_client::WSClient;
use ogn_client::OGNRecord;

/// `Gateway` manages connected websocket clients and distributes
/// `OGNRecord` messages to them.
pub struct Gateway {
    sessions: HashMap<usize, Addr<Syn, WSClient>>,
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
    pub addr: Addr<Syn, WSClient>,
}

impl Handler<Connect> for Gateway {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Client connected ({} clients)", self.sessions.len());

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

        println!("Client disconnected ({} clients)", self.sessions.len());
    }
}

impl Handler<OGNRecord> for Gateway {
    type Result = ();

    fn handle(&mut self, record: OGNRecord, _: &mut Context<Self>) {
        // log record to the console
        println!("{:.4} {:.4} {}", record.record.longitude, record.record.latitude, record.record.raw);

        // distribute record to all connected websocket clients
        for addr in self.sessions.values() {
            addr.do_send(record.clone());
        }
    }
}
