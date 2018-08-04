use actix::prelude::*;
use r2d2::Pool;
use r2d2_redis::RedisConnectionManager;
use _redis::Commands;

use db::models::OGNPosition;

pub struct RedisExecutor {
    pub pool: Pool<RedisConnectionManager>,
}

impl RedisExecutor {
    pub fn new(pool: Pool<RedisConnectionManager>) -> Self {
        RedisExecutor { pool }
    }
}

impl Actor for RedisExecutor {
    type Context = SyncContext<Self>;
}

#[derive(Message)]
pub struct AddOGNPositions {
    pub positions: Vec<OGNPosition>,
}

impl Handler<AddOGNPositions> for RedisExecutor {
    type Result = ();

    fn handle(&mut self, msg: AddOGNPositions, _ctx: &mut Self::Context) -> () {
        let conn = self.pool.get().unwrap();

        for pos in msg.positions {
            let id = format!("ogn_history:{}", pos.ogn_id);

            let data = format!(
                "{:.6}|{:.6}|{}",
                pos.longitude,
                pos.latitude,
                pos.altitude,
            );

            let result: Result<u32, _> = conn.zadd(id, data, pos.time.timestamp());

            if let Err(error) = result {
                error!("Could not create OGN position record in redis: {}", error);
            }
        }
    }
}
