use std::collections::HashMap;
use std::time::Duration;

use actix::prelude::*;
use actix_web::{client, HttpMessage};
use futures::Future;
use serde_json;

use ::redis::*;

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

        // using HTTP because HTTPS needs the `alpn` feature on `actix-web`
        // which can't be compiled on TravisCI right now :(
        client::ClientRequest::get("http://ddb.glidernet.org/download/?j=1&t=1")
            .finish().unwrap()
            .send()
            .map_err(|error| {
                warn!("OGN Device Database download failed: {}", error);
            })
            .and_then(|response| response.json().limit(4_000_000).map_err(|error| {
                error!("OGN Device Database parsing failed: {}", error);
            }))
            .into_actor(self)
            .map(|response: OGNDDBResponse, act, _ctx| {
                let devices: HashMap<_, _> = response.devices.iter()
                    .filter_map(|d| {
                        let id_prefix = match d.device_type.as_ref() {
                            "F" => "FLR",
                            "I" => "ICA",
                            "O" => "OGN",
                            _ => return None,
                        };

                        let ogn_id = format!("{}{}", id_prefix, d.device_id);

                        let category = d.aircraft_type.parse::<i16>();
                        if category.is_err() { return None }
                        let category = category.unwrap();

                        let model = if d.aircraft_model.is_empty() { None } else { Some(d.aircraft_model.clone()) };
                        let registration = if d.registration.is_empty() { None } else { Some(d.registration.clone()) };
                        let callsign = if d.cn.is_empty() { None } else { Some(d.cn.clone()) };

                        Some((ogn_id, DeviceInfo { model, registration, callsign, category }))
                    })
                    .collect();

                match act.redis.try_send(WriteOGNDDB(serde_json::to_string(&devices).unwrap())) {
                    Ok(_) => {
                        debug!("Updated OGN Device Database");
                    }
                    Err(error) => {
                        error!("OGN Device Database update failed: {}", error);
                    }
                }
            })
            .wait(ctx);
        ;
    }
}