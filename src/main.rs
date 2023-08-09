use anyhow::{Context, Ok};
use clap::Parser;
use e57::E57Reader;
use las::Write;
use nalgebra::{Point3, Quaternion, UnitQuaternion, Vector3};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,

    #[arg(short, long, default_value_t = false)]
    should_save: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let input_path = args.path;
    let must_save = args.should_save;

    let mut file = E57Reader::from_file(&input_path).context("Failed to open e57 file")?;

    let pointclouds = file.pointclouds();

    dbg!(pointclouds.len());

    if must_save {
        for (index, pointcloud) in pointclouds.iter().enumerate() {
            let iter = file
                .pointcloud(&pointcloud)
                .context("Unable to get point cloud iterator")?;

            let file_name = input_path.split(".e57").next().unwrap();
            let las_path = format!("{}{}{}", &file_name, index, ".las");
            let transform = pointcloud.transform.clone().unwrap();
            let rotation = UnitQuaternion::from_quaternion(Quaternion::new(
                transform.rotation.w,
                transform.rotation.x,
                transform.rotation.y,
                transform.rotation.z,
            ));
            let translation = Vector3::new(
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
            );
            // let first_point = iter.next().unwrap().unwrap();
            //
            // let e57_point = e57::Point::from_values(first_point, &pointcloud.prototype).unwrap();
            //
            // let x = k

            let mut writer = las::Writer::from_path(&las_path, Default::default())?;

            for p in iter {
                let p = p.context("Unable to read next point")?;

                let p = e57::Point::from_values(p, &pointcloud.prototype)
                    .context("Failed to convert raw point to simple point")?;

                let xyz = if let Some(c) = p.cartesian {
                    if let Some(invalid) = p.cartesian_invalid {
                        if invalid != 0 {
                            continue;
                        }
                    }
                    Point3::new(c.x, c.y, c.z)
                } else if let Some(s) = p.spherical {
                    if let Some(invalid) = p.spherical_invalid {
                        if invalid != 0 {
                            continue;
                        }
                    }
                    let cos_ele = f64::cos(s.elevation);

                    Point3::new(
                        s.range * cos_ele * f64::cos(s.azimuth),
                        s.range * cos_ele * f64::sin(s.azimuth),
                        s.range * f64::sin(s.elevation),
                    )
                } else {
                    // No coordinates found, skip point
                    continue;
                };

                let xyz = rotation.transform_point(&xyz) + translation;

                let las_point = las::Point {
                    x: xyz.x,
                    y: xyz.y,
                    z: xyz.z,
                    ..Default::default()
                };

                // let e57_point = e57::Point::from_values(p.unwrap(), &pointcloud.prototype).unwrap();
                // let CartesianCoordinate { x, y, z } = e57_point.cartesian.unwrap();
                // // let e57::Color { red, green, blue } = e57_point.color.unwrap();
                //
                // let las_point = las::Point {
                //     x,
                //     y,
                //     z,
                //     ..Default::default()
                // };
                //
                writer.write(las_point).unwrap();
            }

            writer.close().unwrap();

            println!("Saved pointcloud {} in {}", index, las_path);
        }
    }

    println!("Finished convertion from e57 to las !");

    Ok(())
}
