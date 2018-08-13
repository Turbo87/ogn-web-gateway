use regex::Regex;

#[derive(Debug)]
pub struct BoundingBox {
    bottom: f64,
    left: f64,
    top: f64,
    right: f64,
}

impl BoundingBox {
    pub fn try_parse(text: &str) -> Option<BoundingBox> {
        lazy_static! {
            static ref RE: Regex = Regex::new(
                r#"(?x)
                ^                           # start at the beginning of the string
                (?P<left>-?\d+(?:\.\d*)?)   # left side of the bounding box in degrees
                \|                          # separator
                (?P<bottom>-?\d+(?:\.\d*)?) # bottom side of the bounding box in degrees
                \|                          # separator
                (?P<right>-?\d+(?:\.\d*)?)  # right side of the bounding box in degrees
                \|                          # separator
                (?P<top>-?\d+(?:\.\d*)?)    # top side of the bounding box in degrees
            "#
            ).unwrap();
        }

        RE.captures(text).and_then(|caps| {
            let left = caps.name("left").unwrap().as_str().parse::<f64>().unwrap();
            let bottom = caps
                .name("bottom")
                .unwrap()
                .as_str()
                .parse::<f64>()
                .unwrap();
            let right = caps.name("right").unwrap().as_str().parse::<f64>().unwrap();
            let top = caps.name("top").unwrap().as_str().parse::<f64>().unwrap();

            if left < -180. || left > 180. || right < -180. || right > 180. {
                return None;
            }

            if top < -90. || top > 90. || bottom < -90. || bottom > 90. || top < bottom {
                return None;
            }

            Some(BoundingBox {
                left,
                bottom,
                right,
                top,
            })
        })
    }

    pub fn contains(&self, longitude: f64, latitude: f64) -> bool {
        latitude <= self.top && latitude >= self.bottom && (if self.left > self.right {
            longitude >= self.left || longitude <= self.right
        } else {
            longitude >= self.left && longitude <= self.right
        })
    }
}

#[cfg(test)]
mod tests {
    use super::BoundingBox;

    #[test]
    fn test_parse_valid() {
        let result = BoundingBox::try_parse("-5.123|42.987|7.|50.3456789");
        assert!(result.is_some());

        let bbox = result.unwrap();
        assert_relative_eq!(bbox.left, -5.123);
        assert_relative_eq!(bbox.bottom, 42.987);
        assert_relative_eq!(bbox.right, 7.);
        assert_relative_eq!(bbox.top, 50.3456789);
    }

    #[test]
    fn test_parse_valid2() {
        let result = BoundingBox::try_parse("5|-2|14|12");
        assert!(result.is_some());

        let bbox = result.unwrap();
        assert_relative_eq!(bbox.left, 5.);
        assert_relative_eq!(bbox.bottom, -2.);
        assert_relative_eq!(bbox.right, 14.);
        assert_relative_eq!(bbox.top, 12.);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(BoundingBox::try_parse("-195.123|42.987|7.|50.3456789").is_none());
        assert!(BoundingBox::try_parse("-5.123|92.987|7.|50.3456789").is_none());
        assert!(BoundingBox::try_parse("-5.123|42.987|197.|50.3456789").is_none());
        assert!(BoundingBox::try_parse("-5.123|42.987|7.|90.3456789").is_none());

        assert!(BoundingBox::try_parse("-5.123|42.987|7.|40.3456789").is_none());

        assert!(BoundingBox::try_parse(".123|42.987|7.|50.3456789").is_none());
        assert!(BoundingBox::try_parse("-5.123|242.a987|7.|50.3456789").is_none());
        assert!(BoundingBox::try_parse("-5.123|0x42.987|7.|50.3456789").is_none());
    }

    #[test]
    fn test_contains_basic() {
        let bbox = BoundingBox::try_parse("5|-2|14|12").unwrap();
        assert!(bbox.contains(7., 10.));
        assert!(bbox.contains(5., -2.));
        assert!(bbox.contains(14., 12.));

        assert!(!bbox.contains(3., 10.));
        assert!(!bbox.contains(15., 10.));
        assert!(!bbox.contains(7., -3.));
        assert!(!bbox.contains(7., 13.));
    }

    #[test]
    fn test_contains_wrap_around() {
        let bbox = BoundingBox::try_parse("175|10|-160|12").unwrap();
        assert!(!bbox.contains(174., 11.));
        assert!(bbox.contains(175., 11.));
        assert!(bbox.contains(-179., 11.));
        assert!(bbox.contains(-160., 11.));
        assert!(!bbox.contains(-159., 11.));
    }
}
