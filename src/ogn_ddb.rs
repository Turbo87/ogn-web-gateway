use std::collections::HashMap;
use std::time::Duration;

use actix::prelude::*;
use actix_web::client::Client;
use futures::Future;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

use crate::redis::*;

pub struct OGNDevicesUpdater {
    pub redis: Addr<RedisExecutor>,
}

impl Actor for OGNDevicesUpdater {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.notify(Update);

        ctx.run_interval(Duration::from_secs(3 * 60 * 60), |_act, ctx| {
            ctx.notify(Update);
        });
    }
}

#[derive(Debug, Deserialize)]
struct OGNDDBResponse {
    devices: Vec<OGNDDBRecord>,
}

#[derive(Debug, Deserialize)]
struct OGNDDBRecord {
    device_type: String,
    device_id: String,
    aircraft_model: String,
    aircraft_type: String,
    registration: String,
    cn: String,
    tracked: String,
    identified: String,
}

impl OGNDDBRecord {
    pub fn ogn_id(&self) -> Option<String> {
        let id_prefix = match self.device_type.as_ref() {
            "F" => "FLR",
            "I" => "ICA",
            "O" => "OGN",
            _ => return None,
        };

        Some(format!("{}{}", id_prefix, self.device_id))
    }
}

#[derive(Serialize)]
struct DeviceInfo {
    pub model: Option<String>,
    pub registration: Option<String>,
    pub callsign: Option<String>,

    /**
     * - 1: Gliders/Motorgliders
     * - 2: Planes
     * - 3: Ultralights
     * - 4: Helicopters
     * - 5: Drones/UAV
     * - 6: Others
     */
    pub category: i16,
}

#[derive(Message)]
struct Update;

impl Handler<Update> for OGNDevicesUpdater {
    type Result = ();

    fn handle(&mut self, _msg: Update, ctx: &mut Self::Context) {
        info!("Downloading OGN Device Database…");

        Client::default()
            .get("https://ddb.glidernet.org/download/?j=1&t=1")
            .send()
            .map_err(|error| {
                warn!("OGN Device Database download failed: {}", error);
            })
            .and_then(|mut response| {
                response.json().limit(4_000_000).map_err(|error| {
                    error!("OGN Device Database parsing failed: {}", error);
                })
            })
            .into_actor(self)
            .map(|response: OGNDDBResponse, act, _ctx| {
                let devices: HashMap<_, _> = response
                    .devices
                    .iter()
                    .filter_map(|d| {
                        let ogn_id = d.ogn_id();
                        if ogn_id.is_none() {
                            return None;
                        }

                        let category = d.aircraft_type.parse::<i16>();
                        if category.is_err() {
                            return None;
                        }
                        let category = category.unwrap();

                        let model = if d.aircraft_model.is_empty() {
                            None
                        } else {
                            Some(d.aircraft_model.clone())
                        };
                        let registration = if d.registration.is_empty() {
                            None
                        } else {
                            Some(d.registration.clone())
                        };
                        let callsign = if d.cn.is_empty() {
                            None
                        } else {
                            Some(d.cn.clone())
                        };

                        Some((
                            ogn_id.unwrap(),
                            DeviceInfo {
                                model,
                                registration,
                                callsign,
                                category,
                            },
                        ))
                    })
                    .collect();

                match act
                    .redis
                    .try_send(WriteOGNDDB(serde_json::to_string(&devices).unwrap()))
                {
                    Ok(_) => {
                        debug!("Updated OGN Device Database");
                    }
                    Err(error) => {
                        error!("OGN Device Database update failed: {}", error);
                    }
                }

                let ignored_device_ids = response
                    .devices
                    .iter()
                    .filter_map(|d| {
                        let ogn_id = d.ogn_id();
                        if ogn_id.is_none() {
                            return None;
                        }

                        if d.tracked != "N" {
                            return None;
                        }

                        Some(ogn_id.unwrap())
                    })
                    .collect::<Vec<_>>();

                match act.redis.try_send(WriteOGNIgnore(ignored_device_ids)) {
                    Ok(_) => {
                        debug!("Updated OGN ignore list");
                    }
                    Err(error) => {
                        error!("OGN ignore list update failed: {}", error);
                    }
                }
            })
            .wait(ctx);
    }
}
