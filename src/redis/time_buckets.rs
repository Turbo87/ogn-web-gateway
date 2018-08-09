use chrono::prelude::*;

pub fn bucket_times_between(from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<i64> {
    let from_bucket_time = from.to_bucket_time();
    let to_bucket_time = to.to_bucket_time();
    (from_bucket_time..=to_bucket_time).step_by(60 * 60).collect()
}

pub trait ToBucketTime {
    fn to_bucket_time(&self) -> i64;
}

impl ToBucketTime for DateTime<Utc> {
    fn to_bucket_time(&self) -> i64 {
        self.with_minute(0).unwrap().with_second(0).unwrap().timestamp()
    }
}

#[cfg(test)]
mod tests {
    use chrono::prelude::*;
    use super::bucket_times_between;

    #[test]
    fn test_bucket_times() {
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
