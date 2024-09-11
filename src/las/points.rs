use uuid::Uuid;

pub trait LasPointsExt {
    fn x_has_color(&self) -> bool;

    fn x_max_cartesian(&self) -> f64;

    fn x_header_builder(&self) -> las::Builder;
}

impl LasPointsExt for Vec<las::Point> {
    fn x_has_color(&self) -> bool {
        self.iter().any(|p| p.color.is_some())
    }

    fn x_max_cartesian(&self) -> f64 {
        self.iter()
            .fold(f64::NEG_INFINITY, |c, p| c.max(p.x).max(p.y).max(p.z))
    }

    fn x_header_builder(&self) -> las::Builder {
        let mut builder = las::Builder::default();

        builder.point_format.has_color = self.x_has_color();
        builder.guid = Uuid::new_v4();

        let offset = 0.0;
        let scale = find_smallest_scale(self.x_max_cartesian());
        let transform = las::Transform { scale, offset };
        builder.transforms = las::Vector {
            x: transform,
            y: transform,
            z: transform,
        };

        builder
    }
}

fn find_smallest_scale(x: f64) -> f64 {
    let mut scale = 0.001;
    let min_i32 = f64::from(i32::MIN);
    let max_i32 = f64::from(i32::MAX);

    while (x / scale).round() < min_i32 || (x / scale).round() > max_i32 {
        scale += 0.0001;
    }

    scale
}
