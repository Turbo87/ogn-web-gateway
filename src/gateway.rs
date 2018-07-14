use chrono;
use actix::prelude::*;
use std::collections::*;
use std::time::Duration;

use aprs;
use ws_client::{WSClient, SendText};
use actix_ogn::OGNMessage;
use geo::BoundingBox;
use time::time_to_datetime;

use db::{DbExecutor, DropOldOGNPositions, CreateOGNPositions};
use db::models::OGNPosition;

/// `Gateway` manages connected websocket clients and distributes
/// `OGNRecord` messages to them.
pub struct Gateway {
    db: Addr<DbExecutor>,
    ws_clients: HashSet<Addr<WSClient>>,
    id_subscriptions: HashMap<String, Vec<Addr<WSClient>>>,
    bbox_subscriptions: HashMap<Addr<WSClient>, BoundingBox>,
    db_buffer: Vec<OGNPosition>,
}

impl Gateway {
    pub fn new(db: Addr<DbExecutor>) -> Gateway {
        Gateway {
            db,
            ws_clients: HashSet::new(),
            id_subscriptions: HashMap::new(),
            bbox_subscriptions: HashMap::new(),
            db_buffer: Vec::new(),
        }
    }

    fn schedule_db_flush(ctx: &mut Context<Self>) {
        ctx.run_later(Duration::from_secs(5), |act, ctx| {
            let buffer = act.db_buffer.split_off(0);

            let count = buffer.len();
            if count > 0 {
                match act.db.try_send(CreateOGNPositions { positions: buffer }) {
                    Ok(_) => {
                        debug!("Flushed {} OGN position records to the database", count);
                    }
                    Err(error) => {
                        error!("Could not flush new OGN position records to the database: {}", error);
                    }
                }
            }

            Self::schedule_db_flush(ctx);
        });
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
        Self::schedule_db_flush(ctx);
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
            users: self.ws_clients.len()
        })
    }
}

/// New websocket client has connected.
#[derive(Message)]
pub struct Connect {
    pub addr: Addr<WSClient>,
}

impl Handler<Connect> for Gateway {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) {
        // register session with random id
        self.ws_clients.insert(msg.addr);

        debug!("Client connected ({} clients)", self.ws_clients.len());
    }
}

/// Websocket client has disconnected.
#[derive(Message)]
pub struct Disconnect {
    pub addr: Addr<WSClient>,
}

impl Handler<Disconnect> for Gateway {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.id_subscriptions.values_mut().for_each(|subscribers| {
            if let Some(pos) = subscribers.iter().position(|x| *x == msg.addr) {
                subscribers.remove(pos);
            }
        });

        self.ws_clients.remove(&msg.addr);

        debug!("Client disconnected ({} clients)", self.ws_clients.len());
    }
}

#[derive(Message)]
pub struct SubscribeToId {
    pub id: String,
    pub addr: Addr<WSClient>,
}

impl Handler<SubscribeToId> for Gateway {
    type Result = ();

    fn handle(&mut self, msg: SubscribeToId, _ctx: &mut Context<Self>) {
        self.id_subscriptions.entry(msg.id)
            .or_insert_with(|| Vec::new())
            .push(msg.addr);
    }
}


#[derive(Message)]
pub struct UnsubscribeFromId {
    pub id: String,
    pub addr: Addr<WSClient>,
}

impl Handler<UnsubscribeFromId> for Gateway {
    type Result = ();

    fn handle(&mut self, msg: UnsubscribeFromId, _ctx: &mut Context<Self>) {
        if let Some(subscribers) = self.id_subscriptions.get_mut(&msg.id) {
            if let Some(pos) = subscribers.iter_mut().position(|x| *x == msg.addr) {
                subscribers.remove(pos);
            }
        }
    }
}


#[derive(Message)]
pub struct SetBoundingBox {
    pub addr: Addr<WSClient>,
    pub bbox: BoundingBox,
}

impl Handler<SetBoundingBox> for Gateway {
    type Result = ();

    fn handle(&mut self, msg: SetBoundingBox, _ctx: &mut Context<Self>) {
        self.bbox_subscriptions.insert(msg.addr, msg.bbox);
    }
}


impl Handler<OGNMessage> for Gateway {
    type Result = ();

    fn handle(&mut self, message: OGNMessage, _: &mut Context<Self>) {
        if let Some(position) = aprs::parse(&message.raw) {
            let now = chrono::Utc::now();
            let time = time_to_datetime(now, position.time);
            let age = time - now;

            // throw away records older than 15min or more than 5min into the future
            if age.num_minutes() > 15 || age.num_minutes() < -5 {
                return;
            }

            // send record to subscribers
            let mut subscribers: Vec<&Addr<WSClient>> = self.bbox_subscriptions.iter()
                .filter(|(_, bbox)| bbox.contains(position.longitude, position.latitude))
                .map(|(addr, _)| addr)
                .collect();

            if let Some(id_subscribers) = self.id_subscriptions.get(position.id) {
                subscribers.extend(id_subscribers);
            }

            if !subscribers.is_empty() {
                let ws_message = format!(
                    "{}|{}|{:.6}|{:.6}|{}",
                    position.id,
                    time.timestamp(),
                    position.longitude,
                    position.latitude,
                    position.course,
                );

                for subscriber in subscribers {
                    subscriber.do_send(SendText(ws_message.clone()));
                }
            }

            // save record in the database
            self.db_buffer.push(OGNPosition {
                ogn_id: position.id.to_owned(),
                time,
                longitude: position.longitude,
                latitude: position.latitude,
                altitude: position.altitude as i32,
            });
        }
    }
}
