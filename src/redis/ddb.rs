use actix::prelude::*;
use failure::Error;
use r2d2_redis::redis::Commands;

use crate::redis::executor::RedisExecutor;

pub struct ReadOGNDDB;

impl Message for ReadOGNDDB {
    type Result = Result<String, Error>;
}

impl Handler<ReadOGNDDB> for RedisExecutor {
    type Result = Result<String, Error>;

    fn handle(&mut self, _msg: ReadOGNDDB, _ctx: &mut Self::Context) -> Self::Result {
        let conn = self.pool.get()?;
        let result: Option<String> = conn.get("ogn-ddb")?;
        Ok(result.unwrap_or_else(|| "{}".to_string()))
    }
}

pub struct WriteOGNDDB(pub String);

impl Message for WriteOGNDDB {
    type Result = Result<(), Error>;
}

impl Handler<WriteOGNDDB> for RedisExecutor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: WriteOGNDDB, _ctx: &mut Self::Context) -> Self::Result {
        let conn = self.pool.get()?;
        conn.set("ogn-ddb", msg.0)?;
        Ok(())
    }
}

pub struct ReadOGNIgnore;

impl Message for ReadOGNIgnore {
    type Result = Result<Vec<String>, Error>;
}

impl Handler<ReadOGNIgnore> for RedisExecutor {
    type Result = Result<Vec<String>, Error>;

    fn handle(&mut self, _msg: ReadOGNIgnore, _ctx: &mut Self::Context) -> Self::Result {
        let conn = self.pool.get()?;
        let result: Option<String> = conn.get("ogn-ignore")?;
        if result.is_none() {
            return Ok(vec![]);
        }

        Ok(serde_json::from_str(&result.unwrap())?)
    }
}

pub struct WriteOGNIgnore(pub Vec<String>);

impl Message for WriteOGNIgnore {
    type Result = Result<(), Error>;
}

impl Handler<WriteOGNIgnore> for RedisExecutor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: WriteOGNIgnore, _ctx: &mut Self::Context) -> Self::Result {
        let conn = self.pool.get()?;
        conn.set("ogn-ignore", serde_json::to_string(&msg.0)?)?;
        Ok(())
    }
}
