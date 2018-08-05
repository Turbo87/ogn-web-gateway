use actix::prelude::*;
use chrono::prelude::*;
use bincode::serialize;
use chrono::{Duration, Utc};
use r2d2::Pool;
use r2d2_redis::RedisConnectionManager;
use _redis::Commands;

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

#[derive(Serialize, Deserialize, Debug)]
pub struct OGNPosition {
    pub longitude: f32,
    pub latitude: f32,
    pub altitude: i16,
}

#[derive(Message)]
pub struct AddOGNPositions {
    pub positions: Vec<(String, DateTime<Utc>, OGNPosition)>,
}

impl Handler<AddOGNPositions> for RedisExecutor {
    type Result = ();

    fn handle(&mut self, msg: AddOGNPositions, _ctx: &mut Self::Context) -> () {
        let conn = self.pool.get().unwrap();

        for (id, time, pos) in msg.positions {
            let id = format!("ogn:{}", id);
            let data = serialize(&pos).unwrap();

            let result: Result<u32, _> = conn.zadd(id, data, time.timestamp());

            if let Err(error) = result {
                error!("Could not create OGN position record in redis: {}", error);
            }
        }
    }
}

pub struct CountOGNPositions;

impl Message for CountOGNPositions {
    type Result = Option<u64>;
}

impl Handler<CountOGNPositions> for RedisExecutor {
    type Result = Option<u64>;

    fn handle(&mut self, _msg: CountOGNPositions, _ctx: &mut Self::Context) -> Self::Result {
        let conn = self.pool.get().unwrap();

        let result = conn.keys("ogn:*").map(|keys: Vec<String>| {
            keys.iter().map(|key| conn.zcard(key).unwrap_or(0)).sum::<u64>()
        });

        match result {
            Ok(num_records) => Some(num_records),
            Err(error) => {
                error!("Could not count OGN position records: {}", error);
                None
            },
        }
    }
}

#[derive(Message)]
pub struct DropOldOGNPositions;

impl Handler<DropOldOGNPositions> for RedisExecutor {
    type Result = ();

    fn handle(&mut self, _msg: DropOldOGNPositions, _ctx: &mut Self::Context) -> () {
        let conn = self.pool.get().unwrap();

        info!("Dropping outdated OGN position records from redis…");

        let now = Utc::now();
        let cutoff_date = now - Duration::days(1);
        let max = cutoff_date.timestamp();

        let result = conn.keys("ogn:*").map(|keys: Vec<String>| {
            keys.iter().map(|key| conn.zrembyscore(key, 0, max).unwrap_or_else(|error| {
                error!("Could not remove old OGN position records for {}: {}", key, error);
                0
            })).sum::<u64>()
        });

        match result {
            Ok(num) => {
                info!("Removed {} old OGN position records from Redis", num);
            },
            Err(error) => {
                error!("Could not read OGN position records keys: {}", error);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;
    use bincode::{serialize, deserialize};

    #[derive(Serialize, Deserialize, Debug)]
    pub struct RedisOGNRecord {
        pub seconds: i16,
        pub altitude: i16,
        pub longitude: f32,
        pub latitude: f32,
    }

    #[test]
    fn test_deserialization() {
        let record1 = RedisOGNRecord {
            seconds: 123,
            altitude: 1234,
            longitude: 52.987,
            latitude: 7.456,
        };

        let record2 = RedisOGNRecord {
            seconds: 234,
            altitude: 2345,
            longitude: 51.987,
            latitude: 7.356,
        };

        let record3 = RedisOGNRecord {
            seconds: 345,
            altitude: 678,
            longitude: 50.987,
            latitude: 7.256,
        };

        let mut vec1 = serialize(&record1).unwrap();
        let mut vec2 = serialize(&record2).unwrap();
        let mut vec3 = serialize(&record3).unwrap();

        vec1.append(&mut vec2);
        vec1.append(&mut vec3);

        let records: Vec<RedisOGNRecord> = vec1.chunks(size_of::<RedisOGNRecord>())
            .map(|it| deserialize(it).unwrap())
            .collect();

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].seconds, 123);
        assert_eq!(records[1].altitude, 2345);
    }
}
