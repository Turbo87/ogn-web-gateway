use std::collections::*;
use std::iter::FromIterator;
use std::time::Duration;

use actix::prelude::*;
use actix_ogn::OGNMessage;
use chrono::prelude::*;
use futures::Future;
use log::{debug, error, warn};

use crate::geo::BoundingBox;
use crate::ogn;
use crate::redis::{self, RedisExecutor};
use crate::ws_client::{SendTextFast, SendTextSlow, WSClient};

/// `Gateway` manages connected websocket clients and distributes
/// `OGNRecord` messages to them.
pub struct Gateway {
    redis: Addr<RedisExecutor>,
    ws_clients: HashSet<Addr<WSClient>>,
    id_subscriptions: HashMap<String, Vec<Addr<WSClient>>>,
    bbox_subscriptions: HashMap<Addr<WSClient>, BoundingBox>,
    ignore_list: HashSet<String>,
    redis_buffer: Vec<(String, redis::OGNPosition)>,
    record_count: Option<u64>,
}

impl Gateway {
    pub fn new(redis: Addr<RedisExecutor>) -> Gateway {
        Gateway {
            redis,
            ws_clients: HashSet::new(),
            id_subscriptions: HashMap::new(),
            bbox_subscriptions: HashMap::new(),
            ignore_list: HashSet::new(),
            redis_buffer: Vec::new(),
            record_count: None,
        }
    }

    fn update_record_count(&self, ctx: &mut Context<Self>) {
        let fut = self
            .redis
            .send(redis::CountOGNPositions)
            .map_err(|error| {
                warn!("Could not count OGN position records in redis: {}", error);
            })
            .into_actor(self)
            .and_then(|result, act, _ctx| {
                if act.record_count.is_none() || result.is_ok() {
                    act.record_count = result.ok();
                }

                fut::ok::<(), (), Self>(())
            });

        ctx.spawn(fut);
    }

    fn flush_records(&mut self, ctx: &mut Context<Self>) {
        let buffer = self.redis_buffer.split_off(0);

        let count = buffer.len();
        if count > 0 {
            let fut = self
                .redis
                .send(redis::AddOGNPositions { positions: buffer })
                .map_err(|error| {
                    error!(
                        "Could not flush new OGN position records to redis: {}",
                        error
                    )
                })
                .into_actor(self)
                .and_then(move |result, act, _ctx| {
                    match result {
                        Ok(()) => {
                            debug!("Flushed {} OGN position records to redis", &count);
                            if act.record_count.is_some() {
                                act.record_count = Some(act.record_count.unwrap() + count as u64);
                            }
                        }
                        Err(error) => error!(
                            "Could not flush new OGN position records to redis: {}",
                            error
                        ),
                    };

                    fut::ok::<(), (), Self>(())
                });

            ctx.spawn(fut);
        }
    }

    fn drop_outdated_records(&self, ctx: &mut Context<Self>) {
        let fut = self
            .redis
            .send(redis::DropOldOGNPositions)
            .map_err(|error| {
                warn!(
                    "Could not drop outdated OGN position records from redis: {}",
                    error
                );
            })
            .into_actor(self)
            .and_then(|result, act, _ctx| {
                if act.record_count.is_some() && result.is_ok() {
                    act.record_count = Some(act.record_count.unwrap() + result.unwrap());
                }

                fut::ok::<(), (), Self>(())
            });

        ctx.spawn(fut);
    }

    fn update_ignore_list(&self, ctx: &mut Context<Self>) {
        let fut = self
            .redis
            .send(redis::ReadOGNIgnore)
            .map_err(|error| {
                warn!("Could not read OGN ignore list from redis: {}", error);
            })
            .into_actor(self)
            .and_then(|result, act, _ctx| {
                if result.is_ok() {
                    act.ignore_list = HashSet::from_iter(result.unwrap());
                    debug!(
                        "Updated OGN ignore list from redis: {} records",
                        act.ignore_list.len()
                    );
                }

                fut::ok::<(), (), Self>(())
            });

        ctx.spawn(fut);
    }
}

impl Actor for Gateway {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.update_record_count(ctx);

        ctx.run_interval(Duration::from_secs(30 * 60), |act, ctx| {
            act.update_record_count(ctx);
        });

        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            act.flush_records(ctx);
        });

        ctx.run_later(Duration::from_secs(30), |act, ctx| {
            act.drop_outdated_records(ctx);

            ctx.run_interval(Duration::from_secs(30 * 60), |act, ctx| {
                act.drop_outdated_records(ctx);
            });
        });

        ctx.run_later(Duration::from_secs(10), |act, ctx| {
            act.update_ignore_list(ctx);

            ctx.run_interval(Duration::from_secs(10 * 60), |act, ctx| {
                act.update_ignore_list(ctx);
            });
        });
    }
}

pub struct RequestStatus;

impl Message for RequestStatus {
    type Result = StatusResponse;
}

pub struct StatusResponse {
    pub users: usize,
    pub record_count: Option<u64>,
}

impl Handler<RequestStatus> for Gateway {
    type Result = MessageResult<RequestStatus>;

    fn handle(&mut self, _msg: RequestStatus, _ctx: &mut Context<Self>) -> Self::Result {
        MessageResult(StatusResponse {
            users: self.ws_clients.len(),
            record_count: self.record_count,
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
        self.bbox_subscriptions.remove(&msg.addr);

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
        self.id_subscriptions
            .entry(msg.id)
            .or_insert_with(Vec::new)
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
        if let Some(position) = ogn::aprs::parse(&message.raw) {
            if self.ignore_list.contains(position.id) {
                return;
            }

            let now = Utc::now();
            let time = ogn::time_to_datetime(now, position.time);
            let age = time - now;

            // throw away records older than 15min or more than 5min into the future
            if age.num_minutes() > 15 || age.num_minutes() < -5 {
                return;
            }

            // find subscribers
            let id_subscribers = self.id_subscriptions.get(position.id);

            let bbox_subscribers: Vec<&Addr<WSClient>> = self
                .bbox_subscriptions
                .iter()
                .filter(|(_, bbox)| bbox.contains(position.longitude, position.latitude))
                .map(|(addr, _)| addr)
                .filter(|addr| id_subscribers.map_or(true, |list| !list.contains(addr)))
                .collect();

            // send record to subscribers
            if !bbox_subscribers.is_empty() || id_subscribers.map_or(false, |list| !list.is_empty())
            {
                let ws_message = format!(
                    "{}|{}|{:.6}|{:.6}|{}|{}",
                    position.id,
                    time.timestamp(),
                    position.longitude,
                    position.latitude,
                    position.course,
                    position.altitude as i32,
                );

                for subscriber in bbox_subscribers {
                    subscriber.do_send(SendTextSlow(ws_message.clone()));
                }

                if let Some(id_subscribers) = id_subscribers {
                    for subscriber in id_subscribers {
                        subscriber.do_send(SendTextFast(ws_message.clone()));
                    }
                }
            }

            // save record in the database
            self.redis_buffer.push((
                position.id.to_owned(),
                redis::OGNPosition {
                    time,
                    longitude: position.longitude as f32,
                    latitude: position.latitude as f32,
                    altitude: position.altitude as i16,
                },
            ));
        }
    }
}
