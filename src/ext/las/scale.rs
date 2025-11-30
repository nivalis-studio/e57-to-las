const MAX_COORDINATE_RANGE: f64 = (i32::MAX as f64) - 1.0;
const SCALE_SAFETY_MARGIN: f64 = 1.001;

#[derive(Clone, Copy, Debug, Default)]
pub enum Scale {
    MicroMeter,
    #[default]
    MilliMeter,
    CentiMeter,
    DeciMeter,
    Meter,
    Degree1e7,
    Degree1e8,
    Custom(f64),
}

impl Scale {
    pub fn value(&self) -> f64 {
        match self {
            Scale::MicroMeter => 1e-6,
            Scale::MilliMeter => 1e-3,
            Scale::CentiMeter => 1e-2,
            Scale::DeciMeter => 1e-1,
            Scale::Meter => 1.0,
            Scale::Degree1e7 => 1e-7,
            Scale::Degree1e8 => 1e-8,
            Scale::Custom(v) => *v,
        }
    }

    pub fn safe_value(&self, max_range: f64) -> f64 {
        let desired = self.value();

        if max_range <= 0.0 || !max_range.is_finite() {
            return desired;
        }

        let min_required_scale = max_range / MAX_COORDINATE_RANGE;

        desired.max(min_required_scale * SCALE_SAFETY_MARGIN)
    }
}
