const MAX_COORDINATE_RANGE: f64 = (i32::MAX as f64) - 1.0;
const SCALE_SAFETY_MARGIN: f64 = 1.001;

/// Coordinate scale factor for LAS outputs.
///
/// The scale factor determines the precision of coordinates stored in LAS format.
/// LAS uses integer coordinates internally, which are converted to floating-point
/// using: `coordinate = (integer_value * scale) + offset`.
///
/// Smaller scale values provide higher precision but can only represent smaller
/// coordinate ranges within the LAS format's ±2³¹ integer limit.
///
/// # Scale Selection Guidelines
///
/// Choose the scale based on three factors:
///
/// 1. **Precision needs**: What level of accuracy do you need?
///    - `MicroMeter` (1e-6): Highest precision, suitable for laboratory scans
///    - `MilliMeter` (1e-3): Good for most terrestrial laser scanning
///    - `CentiMeter` (1e-2): Acceptable for many surveying applications
///    - `Meter` (1.0): Lowest precision, suitable for large-scale mapping
///
/// 2. **Coordinate system**: Geographic vs projected coordinates
///    - `Degree1e7` or `Degree1e8`: For lat/lon geographic coordinates
///    - Metric scales: For projected coordinate systems (UTM, State Plane, etc.)
///
/// 3. **Data extent**: Larger areas require larger scales
///    - The library automatically increases the scale if necessary to fit data
///    - Maximum extent at each scale ≈ 4.3 million scale units (±2³¹ integer range)
///
/// # Automatic Scale Adjustment
///
/// The conversion functions automatically increase the scale if the data extent
/// would overflow the LAS coordinate range. A safety margin of 0.1% is applied.
/// The scale is never decreased below the requested value.
///
/// # Examples
///
/// ```
/// use e57_to_las::Scale;
///
/// // High-precision industrial scanning
/// let micro = Scale::MicroMeter;  // 1 micrometer precision
///
/// // Standard terrestrial laser scanning
/// let milli = Scale::MilliMeter;  // 1 millimeter precision (default)
///
/// // Geographic coordinates
/// let geo = Scale::Degree1e7;     // ~11mm precision at equator
///
/// // Custom scale for special requirements
/// let custom = Scale::Custom(0.001); // Same as MilliMeter
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub enum Scale {
    /// 1 micrometer (1e-6 m) precision. Highest precision, smallest representable extent.
    MicroMeter,

    /// 1 millimeter (1e-3 m) precision. Good default for terrestrial laser scanning.
    #[default]
    MilliMeter,

    /// 1 centimeter (1e-2 m) precision. Suitable for surveying and mapping.
    CentiMeter,

    /// 1 decimeter (1e-1 m) precision. Lower precision for large-area scanning.
    DeciMeter,

    /// 1 meter (1.0 m) precision. Lowest precision, largest representable extent.
    Meter,

    /// 1e-7 degree precision. For geographic coordinates (~11mm at equator).
    Degree1e7,

    /// 1e-8 degree precision. Higher precision for geographic coordinates (~1.1mm at equator).
    Degree1e8,

    /// Custom scale factor. Allows any positive scale value.
    Custom(f64),
}

impl Scale {
    /// Get the numeric scale value.
    ///
    /// Returns the scale factor as a floating-point number.
    ///
    /// # Examples
    ///
    /// ```
    /// use e57_to_las::Scale;
    ///
    /// assert_eq!(Scale::MilliMeter.value(), 0.001);
    /// assert_eq!(Scale::Meter.value(), 1.0);
    /// assert_eq!(Scale::Custom(0.5).value(), 0.5);
    /// ```
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

    /// Calculate a safe scale value that can represent the given coordinate range.
    ///
    /// This method ensures the scale is large enough to represent coordinates
    /// spanning `max_range` units from the center point within LAS format limits.
    /// A safety margin of 0.1% is applied.
    ///
    /// The scale is never decreased below the value of `self`.
    ///
    /// # Arguments
    ///
    /// * `max_range` - Maximum distance from center to edge of data extent
    ///
    /// # Returns
    ///
    /// The larger of: the desired scale value or the minimum required scale
    /// to fit the data.
    pub fn safe_value(&self, max_range: f64) -> f64 {
        let desired = self.value();

        if max_range <= 0.0 || !max_range.is_finite() {
            return desired;
        }

        let min_required_scale = max_range / MAX_COORDINATE_RANGE;

        desired.max(min_required_scale * SCALE_SAFETY_MARGIN)
    }
}
