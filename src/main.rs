mod extended_point;
use anyhow::{Context, Result};
use clap::Parser;
use e57::E57Reader;
use extended_point::ExtendedPoint;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use las::Write;
use nalgebra::{Point3, Quaternion, UnitQuaternion, Vector3};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,

    #[arg(short, long, default_value_t = String::from("./"))]
    output: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let input_path = args.path;
    let output_path = args.output;

    let mut e57_reader = E57Reader::from_file(&input_path).context("Failed to open e57 file")?;

    let pointclouds = e57_reader.pointclouds();

    for (index, pointcloud) in pointclouds.iter().enumerate() {
        let las_path = construct_las_path(&input_path, &output_path, index)
            .context("Couldn't create las path.")?;
        let transform = get_transform(&pointcloud);
        let (rotation, translation) = get_rotations_and_translations(&transform);

        let mut builder = las::Builder::from((1, 2));
        builder.point_format.has_color = true;
        let header = builder.into_header()?;

        let mut writer = las::Writer::from_path(&las_path, header)?;
        let iter = e57_reader
            .pointcloud(pointcloud)
            .context("Unable to get point cloud iterator")?;

        println!("\nSaving pointcloud {} ...", index);
        let progress_bar = ProgressBar::new(pointcloud.records);
        progress_bar.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {msg} ({eta})",
            )
            .unwrap()
            .with_key(
                "eta",
                |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                    write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
                },
            )
            .progress_chars("=>"),
        );

        for p in iter {
            let p = p.context("Unable to read next point")?;
            let p = e57::Point::from_values(p, &pointcloud.prototype)
                .context("Failed to convert raw point to simple point")?;

            if let Some(xyz) = extract_coordinates(&p) {
                let xyz = rotation.transform_point(&xyz) + translation;
                let las_rgb = ExtendedPoint::from(p.clone()).rgb_color;

                let las_point = las::Point {
                    x: xyz.x,
                    y: xyz.y,
                    z: xyz.z,
                    color: Some(las_rgb),
                    ..Default::default()
                };

                writer.write(las_point)?;
            }

            progress_bar.inc(1);
        }

        writer.close()?;

        progress_bar.finish_with_message("Done");
    }

    println!("Finished convertion from e57 to las !");
    Ok(())
}

fn construct_las_path(input_path: &str, output_path: &str, index: usize) -> Result<PathBuf> {
    let output_dir_path = Path::new(output_path);

    let input_file_name = Path::new(input_path)
        .file_stem()
        .context("Couldn't read file stem.")?
        .to_str()
        .context("Invalid file stem encoding.")?;

    let output_sub_dir_path = output_dir_path.join(input_file_name);

    std::fs::create_dir_all(&output_sub_dir_path).context(format!(
        "Couldn't find or create output dir {}.",
        output_sub_dir_path
            .to_str()
            .context("Invalid output dir path encoding.")?
    ))?;

    let las_path = output_sub_dir_path.join(format!("{}{}", index, ".las"));

    Ok(las_path)
}

fn get_transform(pointcloud: &e57::PointCloud) -> e57::Transform {
    pointcloud.transform.clone().unwrap_or(e57::Transform {
        rotation: e57::Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        translation: e57::Translation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    })
}

fn get_rotations_and_translations(
    transform: &e57::Transform,
) -> (UnitQuaternion<f64>, Vector3<f64>) {
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
    (rotation, translation)
}

fn extract_coordinates(p: &e57::Point) -> Option<Point3<f64>> {
    if let Some(ref c) = p.cartesian {
        if let Some(invalid) = p.cartesian_invalid {
            if invalid != 0 {
                return None;
            }
        }
        Some(Point3::new(c.x, c.y, c.z))
    } else if let Some(ref s) = p.spherical {
        if let Some(invalid) = p.spherical_invalid {
            if invalid != 0 {
                return None;
            }
        }
        let cos_ele = f64::cos(s.elevation);
        Some(Point3::new(
            s.range * cos_ele * f64::cos(s.azimuth),
            s.range * cos_ele * f64::sin(s.azimuth),
            s.range * f64::sin(s.elevation),
        ))
    } else {
        None
    }
}
