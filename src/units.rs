pub trait FeetToMeter {
    fn feet_to_meter(self: Self) -> Self;
}

impl FeetToMeter for f32 {
    fn feet_to_meter(self: f32) -> f32 {
        self * 0.3048
    }
}

impl FeetToMeter for f64 {
    fn feet_to_meter(self: f64) -> f64 {
        self * 0.3048
    }
}

#[cfg(test)]
mod tests {
    use super::FeetToMeter;

    #[test]
    fn test_feet_to_meter_f32() {
        assert_relative_eq!(0f32.feet_to_meter(), 0f32);
        assert_relative_eq!(4500f32.feet_to_meter(), 1371.6f32);
    }

    #[test]
    fn test_feet_to_meter_f64() {
        assert_relative_eq!(0f64.feet_to_meter(), 0f64);
        assert_relative_eq!(4500f64.feet_to_meter(), 1371.6f64);
    }
}
