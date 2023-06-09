use std::collections::HashMap;
use std::mem::size_of;

use actix::prelude::*;
use anyhow::Result;
use bincode::{deserialize, serialize};
use chrono::prelude::*;
use chrono::{Duration, Utc};
use itertools::Itertools;
use lazy_static::lazy_static;
use log::{error, info};
use r2d2_redis::redis::{pipe, Commands, Connection};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::redis::executor::RedisExecutor;
use crate::redis::time_buckets::*;

#[derive(Serialize, Deserialize, Debug)]
struct RedisOGNRecord {
    seconds: u16,
    altitude: i16,
    longitude: f32,
    latitude: f32,
}

#[derive(Debug)]
pub struct OGNPosition {
    pub time: DateTime<Utc>,
    pub longitude: f32,
    pub latitude: f32,
    pub altitude: i16,
}

pub struct AddOGNPositions {
    pub positions: Vec<(String, OGNPosition)>,
}

impl Message for AddOGNPositions {
    type Result = Result<()>;
}

impl Handler<AddOGNPositions> for RedisExecutor {
    type Result = Result<()>;

    fn handle(&mut self, msg: AddOGNPositions, _ctx: &mut Self::Context) -> Self::Result {
        let mut conn = self.pool.get()?;

        let mut appends = HashMap::new();
        for (id, pos) in msg.positions {
            let bucket_time = pos.time.to_bucket_time();
            let seconds = (pos.time.minute() * 60 + pos.time.second()) as u16;

            let value = serialize(&RedisOGNRecord {
                seconds,
                altitude: pos.altitude,
                latitude: pos.latitude,
                longitude: pos.longitude,
            })?;

            appends
                .entry(id)
                .or_insert_with(HashMap::new)
                .entry(bucket_time)
                .or_insert_with(Vec::new)
                .extend(value);
        }

        let mut pipeline = pipe();
        for (id, records) in appends {
            for (bucket_time, records) in records {
                let key = format!("ogn:{}:{}", id, bucket_time);
                pipeline.append(key, records);
            }
        }

        pipeline.query(&mut *conn)?;

        Ok(())
    }
}

pub struct CountOGNPositions;

impl Message for CountOGNPositions {
    type Result = Result<u64>;
}

impl Handler<CountOGNPositions> for RedisExecutor {
    type Result = Result<u64>;

    fn handle(&mut self, _msg: CountOGNPositions, _ctx: &mut Self::Context) -> Self::Result {
        let mut iter_conn = self.pool.get()?;
        let mut conn = self.pool.get()?;

        let mut sum = 0;
        for key in iter_conn.scan_match::<&str, String>("ogn:*:*")? {
            let length: u64 = conn.strlen(key)?;
            sum += length;
        }

        Ok(sum / size_of::<RedisOGNRecord>() as u64)
    }
}

pub struct DropOldOGNPositions;

impl Message for DropOldOGNPositions {
    type Result = Result<u64>;
}

impl Handler<DropOldOGNPositions> for RedisExecutor {
    type Result = Result<u64>;

    fn handle(&mut self, _msg: DropOldOGNPositions, _ctx: &mut Self::Context) -> Self::Result {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"ogn:[^:]+:(?P<bucket_time>\d+)").unwrap();
        }

        let mut iter_conn = self.pool.get()?;
        let mut conn = self.pool.get()?;

        info!("Dropping outdated OGN position records from redis…");

        let now = Utc::now();
        let cutoff_date = now - Duration::days(1);
        let max = cutoff_date.timestamp();

        let iter = iter_conn.scan_match("ogn:*:*");
        if iter.is_err() {
            let error = iter.err().unwrap();
            error!("Could not read OGN position records keys: {}", error);
            return Err(error.into());
        }

        let num_deleted_bytes = iter
            .unwrap()
            .filter(|key: &String| {
                let caps = RE.captures(key);
                if caps.is_none() {
                    return false;
                }
                let caps = caps.unwrap();
                let bucket_time: i64 = caps.name("bucket_time").unwrap().as_str().parse().unwrap();
                bucket_time < max
            })
            .filter_map(|key: String| {
                let strlen_result: Result<u64, _> = conn.strlen(&key);

                let result: Result<u64, _> = conn.del(&key);
                if let Err(error) = result {
                    error!("Could not delete OGN position records: {}", error);
                }

                strlen_result.ok()
            })
            .sum::<u64>();

        let num_deleted = num_deleted_bytes / size_of::<RedisOGNRecord>() as u64;
        info!(
            "Dropped {} outdated OGN position records from redis",
            num_deleted
        );
        Ok(num_deleted)
    }
}

pub struct ReadOGNPositions {
    pub ids: Vec<String>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
}

impl Message for ReadOGNPositions {
    type Result = Result<HashMap<String, Vec<OGNPosition>>>;
}

impl Handler<ReadOGNPositions> for RedisExecutor {
    type Result = Result<HashMap<String, Vec<OGNPosition>>>;

    fn handle(&mut self, msg: ReadOGNPositions, _ctx: &mut Self::Context) -> Self::Result {
        let mut conn = self.pool.get()?;

        let after = msg.after.unwrap_or_else(|| Utc::now() - Duration::days(1));
        let before = msg.before.unwrap_or_else(Utc::now);

        let mut result = HashMap::new();
        for id in msg.ids {
            let records = conn.get_ogn_records(&id, after, before)?;
            result.insert(id, records);
        }

        Ok(result)
    }
}

trait OGNRedisCommands: Commands {
    fn get_ogn_records(
        &mut self,
        id: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<OGNPosition>> {
        let mut result: Vec<OGNPosition> = Vec::new();
        for bucket_time in bucket_times_between(from, to) {
            result.extend(
                self.get_ogn_records_for_bucket(id, bucket_time)?
                    .into_iter()
                    .filter(|it| it.time >= from && it.time <= to),
            );
        }

        result.sort_unstable_by_key(|it| it.time);

        Ok(result)
    }

    fn get_ogn_records_for_bucket(
        &mut self,
        id: &str,
        bucket_time: i64,
    ) -> Result<Vec<OGNPosition>> {
        let key = format!("ogn:{}:{}", id, bucket_time);
        let value: Vec<u8> = self.get(key)?;

        let results_iter = value
            .chunks(size_of::<RedisOGNRecord>())
            .map(|chunk| deserialize::<RedisOGNRecord>(chunk))
            .unique_by(|result| result.as_ref().map(|record| record.seconds).unwrap_or(0));

        let mut vec = Vec::new();
        for result in results_iter {
            let record = result?;
            let timestamp = bucket_time + i64::from(record.seconds);
            let time = Utc.timestamp_opt(timestamp, 0).unwrap();

            vec.push(OGNPosition {
                time,
                latitude: record.latitude,
                longitude: record.longitude,
                altitude: record.altitude,
            });
        }

        Ok(vec)
    }
}

impl OGNRedisCommands for Connection {}

#[cfg(test)]
mod tests {
    use bincode::{deserialize, serialize};
    use std::mem::size_of;

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

        let records: Vec<RedisOGNRecord> = vec1
            .chunks(size_of::<RedisOGNRecord>())
            .map(|it| deserialize(it).unwrap())
            .collect();

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].seconds, 123);
        assert_eq!(records[1].altitude, 2345);
    }
}
