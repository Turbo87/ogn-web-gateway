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
