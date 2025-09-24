pub struct LasFormatAttributes {
    pub has_color: bool,
    pub has_time: bool,
    pub las_version: (u8, u8),
}

pub trait LasFormatExt {
    fn from_attributes(attributes: LasFormatAttributes) -> Self;
}

impl LasFormatExt for las::point::Format {
    fn from_attributes(attributes: LasFormatAttributes) -> Self {
        let LasFormatAttributes {
            has_color,
            has_time,
            las_version,
        } = attributes;

        let format = match (has_color, has_time, las_version >= (1, 4)) {
            (false, false, _) => 0,
            (false, true, _) => 1,
            (true, false, _) => 2,
            (true, true, false) => 3,
            (true, true, true) => 6,
        };

        las::point::Format::new(format).expect("should be valid las point format")
    }
}
