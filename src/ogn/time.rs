use chrono::*;

pub fn time_to_datetime(now: DateTime<Utc>, time: NaiveTime) -> DateTime<Utc> {
    let now = now.naive_utc();
    let datetime = now.date().and_time(time);
    let dt = now - datetime;

    DateTime::<Utc>::from_utc(
        if dt.num_hours() <= -12 {
            datetime - Duration::days(1)
        } else if dt.num_hours() >= 12 {
            datetime + Duration::days(1)
        } else {
            datetime
        },
        Utc,
    )
}

#[cfg(test)]
mod tests {
    use super::time_to_datetime;
    use chrono::*;

    fn run_test(now: &str, time: &str, expected_date: &str) {
        let expected: DateTime<Utc> = format!("{}T{}Z", expected_date, time).parse().unwrap();
        assert_eq!(
            time_to_datetime(now.parse().unwrap(), time.parse().unwrap()),
            expected
        );
    }

    #[test]
    fn test_time_to_datetime_realistic() {
        run_test("2018-07-10T18:15:23Z", "15:06:12", "2018-07-10");
        run_test("2018-07-10T18:15:23Z", "20:06:12", "2018-07-10");
    }

    #[test]
    fn test_time_to_datetime_almost_midnight() {
        run_test("2018-07-10T23:30:00Z", "22:30:00", "2018-07-10");
        run_test("2018-07-10T23:30:00Z", "23:30:00", "2018-07-10");
        run_test("2018-07-10T23:30:00Z", "00:30:00", "2018-07-11");

        run_test("2018-07-10T23:30:00Z", "11:29:00", "2018-07-11");
        run_test("2018-07-10T23:30:00Z", "11:30:00", "2018-07-11");
        run_test("2018-07-10T23:30:00Z", "11:31:00", "2018-07-10");
    }

    #[test]
    fn test_time_to_datetime_after_midnight() {
        run_test("2018-07-11T00:30:00Z", "22:30:00", "2018-07-10");
        run_test("2018-07-11T00:30:00Z", "23:30:00", "2018-07-10");
        run_test("2018-07-11T00:30:00Z", "00:30:00", "2018-07-11");

        run_test("2018-07-11T00:30:00Z", "12:29:00", "2018-07-11");
        run_test("2018-07-11T00:30:00Z", "12:30:00", "2018-07-10");
        run_test("2018-07-11T00:30:00Z", "12:31:00", "2018-07-10");
    }
}
