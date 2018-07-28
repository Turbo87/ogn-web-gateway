use std::vec::Vec;

use actix::prelude::*;
use chrono::prelude::*;

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
pub struct CreateOGNPositions {
    pub positions: Vec<OGNPosition>,
}

impl Handler<CreateOGNPositions> for DbExecutor {
    type Result = ();

    fn handle(&mut self, msg: CreateOGNPositions, _ctx: &mut Self::Context) -> () {
        use db::schema::ogn_positions::dsl::ogn_positions;

        let conn: &PgConnection = &self.pool.get().unwrap();

        let result = diesel::insert_into(ogn_positions)
            .values(&msg.positions)
            .on_conflict_do_nothing()
            .execute(conn);

        if let Err(error) = result {
            error!("Could not create OGN position record in database: {}", error);
        }
    }
}

#[derive(Message)]
pub struct DropOldOGNPositions;

impl Handler<DropOldOGNPositions> for DbExecutor {
    type Result = ();

    fn handle(&mut self, _msg: DropOldOGNPositions, _ctx: &mut Self::Context) -> () {
        let conn: &PgConnection = &self.pool.get().unwrap();

        info!("Dropping outdated OGN position records from the database…");

        let result = diesel::sql_query("SELECT drop_chunks(interval '24 hours', 'ogn_positions');")
            .execute(conn);

        if let Err(error) = result {
            error!("Could not drop outdated OGN position records: {}", error);
        }
    }
}

pub struct CountOGNPositions;

impl Message for CountOGNPositions {
    type Result = Option<i64>;
}

impl Handler<CountOGNPositions> for DbExecutor {
    type Result = Option<i64>;

    fn handle(&mut self, _msg: CountOGNPositions, _ctx: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.pool.get().unwrap();

        let result = diesel::sql_query(r#"
            SELECT row_estimate.row_estimate
                FROM _timescaledb_catalog.hypertable h
                    CROSS JOIN LATERAL (
                        SELECT sum(cl.reltuples) AS row_estimate
                            FROM _timescaledb_catalog.chunk c
                                JOIN pg_class cl ON cl.relname = c.table_name
                            WHERE c.hypertable_id = h.id AND h.table_name = 'ogn_positions'
                            GROUP BY h.schema_name, h.table_name
                    ) row_estimate
        "#).get_result::<RowEstimate>(conn);

        match result {
            Ok(row_estimate) => Some(row_estimate.row_estimate as i64),
            Err(error) => {
                error!("Could not count OGN position records: {}", error);
                None
            },
        }
    }
}

pub struct ReadOGNPositions {
    pub ids: Vec<String>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
}

impl Message for ReadOGNPositions {
    type Result = Option<Vec<OGNPosition>>;
}

impl Handler<ReadOGNPositions> for DbExecutor {
    type Result = Option<Vec<OGNPosition>>;

    fn handle(&mut self, msg: ReadOGNPositions, _ctx: &mut Self::Context) -> Self::Result {
        use diesel::dsl::*;
        use db::schema::ogn_positions::dsl::*;

        let conn: &PgConnection = &self.pool.get().unwrap();

        let query = {
            let mut query = ogn_positions.filter(ogn_id.eq(any(&msg.ids))).into_boxed();

            match (msg.after, msg.before) {
                (Some(after), Some(before)) => { query = query.filter(time.between(after, before)); }
                (Some(after), None) => { query = query.filter(time.ge(after)); }
                (None, Some(before)) => { query = query.filter(time.le(before)); }
                _ => {},
            }

            query.order_by(time)
        };

        let result = query.load::<models::OGNPosition>(conn);

        match result {
            Ok(positions) => Some(positions),
            Err(error) => {
                error!("Could not read OGN position records: {}", error);
                None // TODO this should return an error
            },
        }
    }
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
