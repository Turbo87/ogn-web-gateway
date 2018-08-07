use std::mem::size_of;

use actix::prelude::*;
use chrono::prelude::*;
use bincode::serialize;
use chrono::{Duration, Utc};
use r2d2::Pool;
use r2d2_redis::RedisConnectionManager;
use _redis::Commands;
use regex::Regex;

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
pub struct RedisOGNRecord {
    pub seconds: u16,
    pub altitude: i16,
    pub longitude: f32,
    pub latitude: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OGNPosition {
    pub time: DateTime<Utc>,
    pub longitude: f32,
    pub latitude: f32,
    pub altitude: i16,
}

#[derive(Message)]
pub struct AddOGNPositions {
    pub positions: Vec<(String, OGNPosition)>,
}

impl Handler<AddOGNPositions> for RedisExecutor {
    type Result = ();

    fn handle(&mut self, msg: AddOGNPositions, _ctx: &mut Self::Context) -> () {
        let conn = self.pool.get().unwrap();

        for (id, pos) in msg.positions {
            let bucket_time = pos.time.with_minute(0).unwrap().with_second(0).unwrap();
            let seconds = (pos.time.minute() * 60 + pos.time.second()) as u16;

            let key = format!("ogn:{}:{}", id, bucket_time.timestamp());

            let value = serialize(&RedisOGNRecord {
                seconds,
                altitude: pos.altitude,
                latitude: pos.latitude,
                longitude: pos.longitude,
            }).unwrap();

            let result: Result<u32, _> = conn.append(key, value);

            if let Err(error) = result {
                error!("Could not append OGN position record in redis: {}", error);
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

        let result = conn.scan_match("ogn:*:*").map(|iter| {
            iter.map(|key: String| conn.strlen(key).unwrap_or(0)).sum::<u64>()  / size_of::<RedisOGNRecord>() as u64
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
        lazy_static! {
            static ref RE: Regex = Regex::new(r"ogn:[^:]+:(?P<bucket_time>\d+)").unwrap();
        }

        let conn = self.pool.get().unwrap();

        info!("Dropping outdated OGN position records from redis…");

        let now = Utc::now();
        let cutoff_date = now - Duration::days(1);
        let max = cutoff_date.timestamp();

        let iter = conn.scan_match("ogn:*:*");
        if iter.is_err() {
            error!("Could not read OGN position records keys: {}", iter.err().unwrap());
            return;
        }

        iter.unwrap()
            .filter(|key: &String| {
                let caps = RE.captures(key);
                if caps.is_none() { return false; }
                let caps = caps.unwrap();
                let bucket_time: i64 = caps.name("bucket_time").unwrap().as_str().parse().unwrap();
                return bucket_time < max;
            })
            .for_each(|key: String| {
                let result: Result<u64, _> = conn.del(key);
                if let Err(error) = result {
                    error!("Could not delete OGN position records: {}", error);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;
    use bincode::{serialize, deserialize};

    use super::RedisOGNRecord;

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
