use actix::prelude::*;

use diesel;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

pub mod models;
pub mod schema;

use db::models::CreateOGNPosition;

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

impl Handler<CreateOGNPosition> for DbExecutor {
    type Result = ();

    fn handle(&mut self, data: CreateOGNPosition, _ctx: &mut Self::Context) -> () {
        use db::schema::ogn_positions::dsl::ogn_positions;

        let conn: &PgConnection = &self.pool.get().unwrap();

        let result = diesel::insert_into(ogn_positions)
            .values(&data)
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
        use db::schema::ogn_positions::dsl::ogn_positions;

        let conn: &PgConnection = &self.pool.get().unwrap();

        let result = ogn_positions.count().get_result(conn);

        match result {
            Ok(count) => Some(count),
            Err(error) => {
                error!("Could not count OGN position records: {}", error);
                None
            },
        }
    }
}
