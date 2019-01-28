use chrono::NaiveTime;
use regex::Regex;

use units::FeetToMeter;

pub struct APRSPosition<'a> {
    pub id: &'a str,
    pub time: NaiveTime,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub course: i32,
}

pub fn parse<'a>(line: &'a str) -> Option<APRSPosition<'a>> {
    // Examples:
    // FLRDD9612>APRS,qAS,VillaBlau:/141956h4911.18N/00815.93E'126/059/A=003716 !W75! id06DD9612 -355fpm -1.2rot 3.0dB 2e -1.3kHz gps3x3
    // ICA4060D7>APRS,qAS,UKDUN2:/141953h5147.03N\00109.00W^210/143/A=003405 !W50! id214060D7 +079fpm +0.0rot 8.0dB 0e -11.9kHz gps3x4
    // FLRDD87AC>APRS,qAS,LFQB:/141950h4818.33N/00401.87E'014/034/A=005199 !W26! id06DD87AC +218fpm +2.5rot 17.8dB 0e -2.4kHz gps3x4 -1.0dBm

    lazy_static! {
        static ref RE: Regex = Regex::new(
            r#"(?x)
            (?P<id>[A-Z]{3}[\dA-F]{6}) # sender ID
            [^:]+:                     # header incl. separator
            /                          # position report indicator
            (?P<time>\d{6})h           # time in HHMMSS incl. `h` indicator
            (?P<lat>\d{4}.\d{2})       # latitude angle in DDMM.mm
            (?P<lat_sign>[NS])         # latitude hemisphere
            .                          # symbol table identifier
            (?P<lon>\d{5}.\d{2})       # longitude angle in DDDMM.mm
            (?P<lon_sign>[WE])         # longitude hemisphere
            .                          # symbol code
            (?P<course>\d{3})          # course angle in DDD
            /                          # separator
            (?P<speed>\d{3})           # speed in knots
            /                          # separator
            A=(?P<alt>\d{6})           # altitude in feet (converted to meters)
            (?:\x20!W(?P<ppe>\d\d)!)?  # position precision enhancement
        "#
        )
        .unwrap();
    }

    RE.captures(line).map(|caps| {
        let id = caps.name("id").unwrap().as_str();

        let time = {
            let hhmmss = caps.name("time").unwrap().as_str();
            NaiveTime::parse_from_str(hhmmss, "%H%M%S").unwrap()
        };

        let latitude = {
            let raw_angle = caps.name("lat").unwrap().as_str();
            let mut angle = raw_angle[0..2].parse::<f64>().unwrap()
                + raw_angle[2..].parse::<f64>().unwrap() / 60.;

            if let Some(ppe) = caps.name("ppe") {
                let enhancement = ppe.as_str()[0..1].parse::<f64>().unwrap();
                angle += enhancement / 60000.
            }

            let hemisphere = caps.name("lat_sign").unwrap().as_str();
            if hemisphere == "N" {
                angle
            } else {
                -angle
            }
        };

        let longitude = {
            let raw_angle = caps.name("lon").unwrap().as_str();
            let mut angle = raw_angle[0..3].parse::<f64>().unwrap()
                + raw_angle[3..].parse::<f64>().unwrap() / 60.;

            if let Some(ppe) = caps.name("ppe") {
                let enhancement = ppe.as_str()[1..2].parse::<f64>().unwrap();
                angle += enhancement / 60000.
            }

            let hemisphere = caps.name("lon_sign").unwrap().as_str();
            if hemisphere == "E" {
                angle
            } else {
                -angle
            }
        };

        let course = caps
            .name("course")
            .unwrap()
            .as_str()
            .parse::<i32>()
            .unwrap();

        let altitude = caps
            .name("alt")
            .unwrap()
            .as_str()
            .parse::<f64>()
            .unwrap()
            .feet_to_meter();

        APRSPosition {
            id,
            time,
            latitude,
            longitude,
            altitude,
            course,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn test_parse_1() {
        let result = parse(r"FLRDD9612>APRS,qAS,VillaBlau:/141956h4911.18N/00815.93E'126/059/A=003716 !W75! id06DD9612 -355fpm -1.2rot 3.0dB 2e -1.3kHz gps3x3");
        assert!(result.is_some());

        let position = result.unwrap();
        assert_eq!(position.id, "FLRDD9612");
        assert_eq!(position.time, "14:19:56".parse().unwrap());
        assert_relative_eq!(position.latitude, 49. + 11.187 / 60.);
        assert_relative_eq!(position.longitude, 8. + 15.935 / 60.);
        assert_relative_eq!(position.altitude, 1132.6368);
        assert_eq!(position.course, 126);
    }

    #[test]
    fn test_parse_2() {
        let result = parse(r"ICA4060D7>APRS,qAS,UKDUN2:/141953h5147.03N\00109.00W^210/143/A=003405 !W50! id214060D7 +079fpm +0.0rot 8.0dB 0e -11.9kHz gps3x4");
        assert!(result.is_some());

        let position = result.unwrap();
        assert_eq!(position.id, "ICA4060D7");
        assert_eq!(position.time, "14:19:53".parse().unwrap());
        assert_relative_eq!(position.latitude, 51. + 47.035 / 60.);
        assert_relative_eq!(position.longitude, -(1. + 9.000 / 60.));
        assert_relative_eq!(position.altitude, 1037.844);
        assert_eq!(position.course, 210);
    }

    #[test]
    fn test_parse_3() {
        let result = parse(r"FLRDD87AC>APRS,qAS,LFQB:/141950h4818.33N/00401.87E'014/034/A=005199 id06DD87AC +218fpm +2.5rot 17.8dB 0e -2.4kHz gps3x4 -1.0dBm");
        assert!(result.is_some());

        let position = result.unwrap();
        assert_eq!(position.id, "FLRDD87AC");
        assert_eq!(position.time, "14:19:50".parse().unwrap());
        assert_relative_eq!(position.latitude, 48. + 18.33 / 60.);
        assert_relative_eq!(position.longitude, 4. + 1.87 / 60.);
        assert_relative_eq!(position.altitude, 1584.6552);
        assert_eq!(position.course, 14);
    }
}
