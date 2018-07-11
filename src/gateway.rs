use chrono;
use actix::prelude::*;
use rand::{self, Rng, ThreadRng};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

use aprs;
use ws_client::{WSClient, SendText};
use actix_ogn::OGNMessage;
use time::time_to_datetime;

use db::{DbExecutor, DropOldOGNPositions};
use db::models::CreateOGNPosition;

/// `Gateway` manages connected websocket clients and distributes
/// `OGNRecord` messages to them.
pub struct Gateway {
    db: Addr<DbExecutor>,
    sessions: HashMap<usize, Addr<WSClient>>,
    rng: RefCell<ThreadRng>,
}

impl Gateway {
    pub fn new(db: Addr<DbExecutor>) -> Gateway {
        Gateway {
            db,
            sessions: HashMap::new(),
            rng: RefCell::new(rand::thread_rng()),
        }
    }

    fn schedule_db_cleanup(ctx: &mut Context<Self>) {
        ctx.run_later(Duration::from_secs(30 * 60), |act, ctx| {
            act.db.do_send(DropOldOGNPositions);
            Self::schedule_db_cleanup(ctx);
        });
    }
}

impl Actor for Gateway {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        Self::schedule_db_cleanup(ctx);
    }
}

pub struct RequestStatus;

impl Message for RequestStatus {
    type Result = StatusResponse;
}

pub struct StatusResponse {
    pub users: usize,
}

impl Handler<RequestStatus> for Gateway {
    type Result = MessageResult<RequestStatus>;

    fn handle(&mut self, _msg: RequestStatus, _ctx: &mut Context<Self>) -> Self::Result {
        MessageResult(StatusResponse {
            users: self.sessions.len()
        })
    }
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
                "{}|{}|{:.6}|{:.6}|{}",
                position.id,
                time.timestamp(),
                position.longitude,
                position.latitude,
                position.course,
            );

            // log record to the console
            trace!("{}", ws_message);

            // distribute record to all connected websocket clients
            for addr in self.sessions.values() {
                addr.do_send(SendText(ws_message.clone()));
            }

            self.db.do_send(CreateOGNPosition {
                ogn_id: position.id.to_owned(),
                time,
                longitude: position.longitude,
                latitude: position.latitude,
            })
        }
    }
}
