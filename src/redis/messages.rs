use std::collections::HashMap;
use std::mem::size_of;

use actix::prelude::*;
use chrono::prelude::*;
use bincode::{serialize, deserialize};
use chrono::{Duration, Utc};
use failure::Error;
use _redis::{Commands, Connection};
use regex::Regex;
use itertools::Itertools;

use redis::RedisExecutor;

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

#[derive(Message)]
pub struct AddOGNPositions {
    pub positions: Vec<(String, OGNPosition)>,
}

impl Handler<AddOGNPositions> for RedisExecutor {
    type Result = ();

    fn handle(&mut self, msg: AddOGNPositions, _ctx: &mut Self::Context) -> () {
        let conn = self.pool.get().unwrap();

        for (id, pos) in msg.positions {
            let bucket_time = pos.time.to_bucket_time();
            let seconds = (pos.time.minute() * 60 + pos.time.second()) as u16;

            let key = format!("ogn:{}:{}", id, bucket_time);

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

pub struct ReadOGNPositions {
    pub ids: Vec<String>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
}

impl Message for ReadOGNPositions {
    type Result = Result<HashMap<String, Vec<OGNPosition>>, Error>;
}

impl Handler<ReadOGNPositions> for RedisExecutor {
    type Result = Result<HashMap<String, Vec<OGNPosition>>, Error>;

    fn handle(&mut self, msg: ReadOGNPositions, _ctx: &mut Self::Context) -> Self::Result {
        let conn = self.pool.get().unwrap();

        let after = msg.after.unwrap_or_else(|| Utc::now() - Duration::days(1));
        let before = msg.before.unwrap_or_else(|| Utc::now());

        let mut result = HashMap::new();
        for id in msg.ids {
            let records = conn.get_ogn_records(&id, after, before)?;
            result.insert(id, records);
        }

        Ok(result)
    }
}

trait OGNRedisCommands {
    fn get_ogn_records(&self, id: &str, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<OGNPosition>, Error>;
    fn get_ogn_records_for_bucket(&self, id: &str, bucket_time: i64) -> Result<Vec<OGNPosition>, Error>;
}

impl OGNRedisCommands for Connection {
    fn get_ogn_records(&self, id: &str, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<OGNPosition>, Error> {
        let mut result = Vec::new();
        for bucket_time in bucket_times_between(from, to) {
            result.extend(self.get_ogn_records_for_bucket(id, bucket_time)?);
        }

        result.sort_unstable_by_key(|it| it.time);

        Ok(result)
    }

    fn get_ogn_records_for_bucket(&self, id: &str, bucket_time: i64) -> Result<Vec<OGNPosition>, Error> {
        let key = format!("ogn:{}:{}", id, bucket_time);
        let value: Vec<u8> = self.get(key)?;


        let results_iter = value
            .chunks(size_of::<RedisOGNRecord>())
            .map(|chunk| deserialize::<RedisOGNRecord>(chunk))
            .unique_by(|result| {
                result.as_ref().map(|record| record.seconds).unwrap_or(0)
            });

        let mut vec = Vec::new();
        for result in results_iter {
            let record = result?;
            let timestamp = bucket_time + record.seconds as i64;
            let time = Utc.timestamp(timestamp, 0);

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

fn bucket_times_between(from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<i64> {
    let from_bucket_time = from.to_bucket_time();
    let to_bucket_time = to.to_bucket_time();
    (from_bucket_time..=to_bucket_time).step_by(60 * 60).collect()
}

trait ToBucketTime {
    fn to_bucket_time(&self) -> i64;
}

impl ToBucketTime for DateTime<Utc> {
    fn to_bucket_time(&self) -> i64 {
        self.with_minute(0).unwrap().with_second(0).unwrap().timestamp()
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

    #[test]
    fn test_bucket_times() {
        use chrono::prelude::*;
        use super::bucket_times_between;

        fn check(from: &str, to: &str, expected: Vec<&str>) {
            assert_eq!(bucket_times_between(from.parse().unwrap(), to.parse().unwrap()),
                       expected.iter().map(|it| it.parse::<DateTime<Utc>>().unwrap().timestamp()).collect::<Vec<_>>());
        }

        check("2018-08-07T01:23:45Z", "2018-08-07T01:23:45Z", vec![
            "2018-08-07T01:00:00Z",
        ]);

        check("2018-08-07T01:23:45Z", "2018-08-07T05:00:00Z", vec![
            "2018-08-07T01:00:00Z",
            "2018-08-07T02:00:00Z",
            "2018-08-07T03:00:00Z",
            "2018-08-07T04:00:00Z",
            "2018-08-07T05:00:00Z",
        ]);

        check("2018-08-06T22:00:00Z", "2018-08-07T03:59:59Z", vec![
            "2018-08-06T22:00:00Z",
            "2018-08-06T23:00:00Z",
            "2018-08-07T00:00:00Z",
            "2018-08-07T01:00:00Z",
            "2018-08-07T02:00:00Z",
            "2018-08-07T03:00:00Z",
        ]);
    }
}
