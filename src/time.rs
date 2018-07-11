use chrono::{NaiveDateTime, NaiveTime, Duration};

pub fn time_to_datetime(now: NaiveDateTime, time: NaiveTime) -> NaiveDateTime {
    let datetime = now.date().and_time(time);
    let dt = now  - datetime;

    return if dt.num_hours() <= -12 {
        datetime - Duration::days(1)
    } else if dt.num_hours() >= 12 {
        datetime + Duration::days(1)
    } else {
        datetime
    }
}

#[cfg(test)]
mod tests {
    use super::time_to_datetime;

    fn run_test(now: &str, time: &str, expected_date: &str) {
        let expected = format!("{}T{}", expected_date, time);
        assert_eq!(time_to_datetime(now.parse().unwrap(), time.parse().unwrap()), expected.parse().unwrap());
    }

    #[test]
    fn test_time_to_datetime_realistic() {
        run_test("2018-07-10T18:15:23", "15:06:12", "2018-07-10");
        run_test("2018-07-10T18:15:23", "20:06:12", "2018-07-10");
    }

    #[test]
    fn test_time_to_datetime_almost_midnight() {
        run_test("2018-07-10T23:30:00", "22:30:00", "2018-07-10");
        run_test("2018-07-10T23:30:00", "23:30:00", "2018-07-10");
        run_test("2018-07-10T23:30:00", "00:30:00", "2018-07-11");

        run_test("2018-07-10T23:30:00", "11:29:00", "2018-07-11");
        run_test("2018-07-10T23:30:00", "11:30:00", "2018-07-11");
        run_test("2018-07-10T23:30:00", "11:31:00", "2018-07-10");
    }

    #[test]
    fn test_time_to_datetime_after_midnight() {
        run_test("2018-07-11T00:30:00", "22:30:00", "2018-07-10");
        run_test("2018-07-11T00:30:00", "23:30:00", "2018-07-10");
        run_test("2018-07-11T00:30:00", "00:30:00", "2018-07-11");

        run_test("2018-07-11T00:30:00", "12:29:00", "2018-07-11");
        run_test("2018-07-11T00:30:00", "12:30:00", "2018-07-10");
        run_test("2018-07-11T00:30:00", "12:31:00", "2018-07-10");
    }
}
