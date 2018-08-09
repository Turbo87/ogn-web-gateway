use std::vec::Vec;

use actix::prelude::*;

use diesel;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

pub mod models;
pub mod schema;

use db::models::*;

pub struct DbExecutor {
    pub pool: Pool<ConnectionManager<PgConnection>>,
}

impl DbExecutor {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        DbExecutor { pool }
    }
}

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

#[derive(Message)]
pub struct UpsertOGNDevices {
    pub devices: Vec<OGNDevice>,
}

impl Handler<UpsertOGNDevices> for DbExecutor {
    type Result = ();

    fn handle(&mut self, msg: UpsertOGNDevices, _ctx: &mut Self::Context) -> () {
        use diesel::pg::upsert::excluded;
        use db::schema::ogn_devices::dsl::*;

        let conn: &PgConnection = &self.pool.get().unwrap();

        let result = diesel::insert_into(ogn_devices)
            .values(&msg.devices)
            .on_conflict(ogn_id)
            .do_update()
            .set((
                model.eq(excluded(model)),
                category.eq(excluded(category)),
                registration.eq(excluded(registration)),
                callsign.eq(excluded(callsign)),
            ))
            .execute(conn);

        if let Err(error) = result {
            error!("Could not update OGN device records: {}", error);
        }
    }
}

pub struct ReadOGNDevices;

impl Message for ReadOGNDevices {
    type Result = Option<Vec<OGNDevice>>;
}

impl Handler<ReadOGNDevices> for DbExecutor {
    type Result = Option<Vec<OGNDevice>>;

    fn handle(&mut self, _msg: ReadOGNDevices, _ctx: &mut Self::Context) -> Self::Result {
        use db::schema::ogn_devices::dsl::*;

        let conn: &PgConnection = &self.pool.get().unwrap();

        let result = ogn_devices
            .order_by(ogn_id)
            .load::<OGNDevice>(conn);

        match result {
            Ok(devices) => Some(devices),
            Err(error) => {
                error!("Could not read OGN device records: {}", error);
                None // TODO this should return an error
            },
        }
    }
}
