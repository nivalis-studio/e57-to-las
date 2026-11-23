#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, missing_docs)]

mod common;

use std::sync::{Arc, Mutex};

use common::{Cloud, cloud, fixture, fixture_with, mm, p, read_las, read_las_bytes, sorted_mm};
use e57_to_las::output::{FileOutput, MemoryOutput};
use e57_to_las::{
    Attributes, CancelToken, Converter, Crs, Error, Event, GroupKey, Grouping, Input, LasVersion,
    Precision, Scan, Vlr, Warning,
};

/// A colored, translated cloud and a colorless one.
fn two_clouds() -> Vec<Cloud> {
    vec![
        cloud(vec![
            p(0.0, 0.0, 0.0).color(255, 0, 255),
            p(1.5, -2.0, 0.5).color(0, 255, 0),
            p(-3.0, 4.5, -1.0).color(255, 255, 255),
        ])
        .named("north")
        .translated([10.0, -20.0, 5.5]),
        cloud(vec![p(2.0, 2.5, 3.0), p(-0.5, -1.5, 7.0)]).named("south"),
    ]
}

#[test]
fn merged_output_has_every_point_with_pose_color_and_source() {
    let fx = fixture(&two_clouds());
    let out = fx.path("out.las");
    let summary = e57_to_las::convert(&fx.input, &out).unwrap();

    assert_eq!(summary.points_written(), 5);
    assert_eq!(summary.points_skipped(), 0);
    assert_eq!(summary.outputs.len(), 1);
    assert_eq!(summary.outputs[0].location.as_deref(), Some(out.as_path()));
    assert_eq!(summary.outputs[0].point_format, 2);

    let (header, points) = read_las(&out);
    assert_eq!(header.version(), las::Version::new(1, 4));
    assert_eq!(header.number_of_points(), 5);
    let mut got: Vec<_> = points
        .iter()
        .map(|p| {
            (
                mm(p),
                p.color.map(|c| (c.red, c.green, c.blue)),
                p.point_source_id,
            )
        })
        .collect();
    got.sort_unstable();
    let full = u16::MAX;
    let mut expected = vec![
        ([10_000, -20_000, 5_500], Some((full, 0, full)), 0),
        ([11_500, -22_000, 6_000], Some((0, full, 0)), 0),
        ([7_000, -15_500, 4_500], Some((full, full, full)), 0),
        ([2_000, 2_500, 3_000], Some((0, 0, 0)), 1),
        ([-500, -1_500, 7_000], Some((0, 0, 0)), 1),
    ];
    expected.sort_unstable();
    assert_eq!(got, expected);
    assert_eq!(
        std::fs::read_dir(fx.dir.path()).unwrap().count(),
        2,
        "only input.e57 and out.las"
    );
}

#[test]
fn per_cloud_outputs_follow_the_template() {
    let fx = fixture(&two_clouds());
    let summary = Converter::new()
        .grouping(Grouping::PerCloud)
        .convert(
            &fx.input,
            FileOutput::new(fx.path("las/{stem}_{index}_{name}.las")),
        )
        .unwrap();

    assert_eq!(
        fx.files(),
        [
            "input.e57",
            "las/input_0_north.las",
            "las/input_1_south.las"
        ]
    );
    let keys: Vec<_> = summary.outputs.iter().map(|o| o.key.to_string()).collect();
    assert_eq!(keys, ["0", "1"]);
    let (north, points) = read_las(&fx.path("las/input_0_north.las"));
    assert_eq!(points.len(), 3);
    assert!(north.point_format().has_color);
    let (south, points) = read_las(&fx.path("las/input_1_south.las"));
    assert_eq!(points.len(), 2);
    assert!(
        !south.point_format().has_color,
        "per-cloud output only has its own attributes"
    );
    assert_eq!(
        sorted_mm(&points),
        [[-500, -1_500, 7_000], [2_000, 2_500, 3_000]]
    );
}

#[test]
fn stable_order_writes_clouds_in_file_order() {
    let clouds: Vec<Cloud> = (0..4)
        .map(|c| {
            cloud(
                (0..20_000)
                    .map(|i| p(i as f64 * 0.01, c as f64, 0.0))
                    .collect(),
            )
        })
        .collect();
    let fx = fixture(&clouds);
    for threads in [1, 4] {
        let out = fx.path(&format!("out_{threads}.las"));
        Converter::new()
            .threads(threads)
            .stable_order(true)
            .convert(&fx.input, FileOutput::new(&out))
            .unwrap();
        let (_, points) = read_las(&out);
        let expected: Vec<[i64; 3]> = (0..4)
            .flat_map(|c| (0..20_000).map(move |i| [i * 10, c * 1000, 0]))
            .collect();
        assert_eq!(points.iter().map(mm).collect::<Vec<_>>(), expected);
    }
}

#[test]
fn parallel_merged_output_holds_every_point_once() {
    // More points than one batch, in several clouds.
    let clouds: Vec<Cloud> = (0..5)
        .map(|c| {
            cloud(
                (0..70_000)
                    .map(|i| p(i as f64 * 0.001, c as f64, 1.0))
                    .collect(),
            )
        })
        .collect();
    let fx = fixture(&clouds);
    let out = fx.path("out.las");
    let summary = Converter::new()
        .threads(4)
        .convert(&fx.input, FileOutput::new(&out))
        .unwrap();
    assert_eq!(summary.points_written(), 350_000);
    let (_, points) = read_las(&out);
    let expected: Vec<[i64; 3]> = (0..5)
        .flat_map(|c| (0..70_000).map(move |i| [i, c * 1000, 1000]))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(sorted_mm(&points), expected);
}

#[test]
fn invalid_and_non_finite_points_are_skipped_and_counted() {
    let fx = fixture(&[cloud(vec![
        p(1.0, 2.0, 3.0),
        p(f64::NAN, 0.0, 0.0),
        p(0.0, f64::INFINITY, 0.0),
        p(9.0, 9.0, 9.0).invalid(),
        p(4.0, 5.0, 6.0),
    ])]);
    let out = fx.path("out.las");
    let summary = e57_to_las::convert(&fx.input, &out).unwrap();
    assert_eq!(summary.points_written(), 2);
    assert_eq!(summary.points_skipped(), 3);
    let (header, points) = read_las(&out);
    assert_eq!(sorted_mm(&points), [[1000, 2000, 3000], [4000, 5000, 6000]]);
    assert_eq!(header.transforms().x.scale, 0.001);
}

#[test]
fn georeferenced_points_keep_millimeters() {
    let fx = fixture(&[cloud(vec![
        p(651_234.567, 6_862_345.678, 41.234),
        p(651_334.001, 6_862_445.002, 45.999),
    ])]);
    let out = fx.path("out.las");
    e57_to_las::convert(&fx.input, &out).unwrap();
    let (header, points) = read_las(&out);
    assert_eq!(header.transforms().x.scale, 0.001);
    assert_eq!(header.transforms().y.scale, 0.001);
    assert_eq!(
        sorted_mm(&points),
        [
            [651_234_567, 6_862_345_678, 41_234],
            [651_334_001, 6_862_445_002, 45_999]
        ]
    );
}

#[test]
fn far_points_restart_the_output_with_a_coarser_scale() {
    // Correct bounds metadata: the coarse scale is chosen up front.
    let fx = fixture(&[
        cloud(vec![p(0.0, 0.0, 0.0), p(1.0, 1.0, 1.0)]),
        cloud(vec![p(5.0e6, 0.0, 0.0), p(-5.0e6, 2.0, 2.0)]),
    ]);
    // Wrong bounds metadata: the fast path fails and the output is restarted.
    let fx_lying = fixture(&[
        cloud(vec![p(0.0, 0.0, 0.0), p(1.0, 1.0, 1.0)]).bounds([0.0; 3], [1.0; 3]),
        cloud(vec![p(5.0e6, 0.0, 0.0), p(-5.0e6, 2.0, 2.0)]).bounds([0.0; 3], [1.0; 3]),
    ]);
    for (fx, restarts) in [(fx, 0), (fx_lying, 1)] {
        let out = fx.path("out.las");
        let restarted = Arc::new(Mutex::new(0));
        let seen = Arc::clone(&restarted);
        let summary = Converter::new()
            .threads(1)
            .observer(move |event| {
                if let Event::OutputRestarted { .. } = event {
                    *seen.lock().unwrap() += 1;
                }
            })
            .convert(&fx.input, FileOutput::new(&out))
            .unwrap();
        assert_eq!(summary.points_written(), 4);
        assert!(
            summary
                .warnings
                .iter()
                .any(|w| matches!(w, Warning::CoarseScale { .. })),
            "{:?}",
            summary.warnings
        );
        let (header, points) = read_las(&out);
        assert_eq!(points.len(), 4);
        assert!(header.transforms().x.scale > 0.001);
        assert_eq!(header.transforms().y.scale, 0.001);
        assert!(points.iter().any(|p| (p.x - 5.0e6).abs() < 0.01));
        assert_eq!(
            fx.files(),
            ["input.e57", "out.las"],
            "no temporary file left"
        );
        assert_eq!(*restarted.lock().unwrap(), restarts);
    }
}

#[test]
fn metadata_bounds_give_the_offset() {
    let fx = fixture(&[cloud(vec![p(100.0, 200.0, 300.0), p(110.0, 220.0, 330.0)])]);
    let out = fx.path("out.las");
    let summary = e57_to_las::convert(&fx.input, &out).unwrap();
    assert_eq!(summary.outputs[0].offset, [105.0, 210.0, 315.0]);
}

#[test]
fn fixed_precision_rejects_points_that_do_not_fit() {
    let fx = fixture(&[cloud(vec![p(0.0, 0.0, 0.0), p(3.0e6, 0.0, 0.0)])]);
    let err = Converter::new()
        .precision(Precision::Fixed {
            scale: [0.001; 3],
            offset: [0.0; 3],
        })
        .convert(&fx.input, FileOutput::new(fx.path("out.las")))
        .unwrap_err();
    assert!(
        matches!(err, Error::PointOutOfRange { cloud: 0, .. }),
        "{err:?}"
    );
    assert_eq!(fx.files(), ["input.e57"]);
}

#[test]
fn colors_need_las_1_2_unless_disabled() {
    let fx = fixture(&two_clouds());
    for version in [LasVersion::V1_0, LasVersion::V1_1] {
        let err = Converter::new()
            .las_version(version)
            .convert(&fx.input, FileOutput::new(fx.path("out.las")))
            .unwrap_err();
        assert!(matches!(err, Error::ColorUnsupported { .. }));
        let summary = Converter::new()
            .las_version(version)
            .attributes(Attributes::default().color(false))
            .convert(&fx.input, FileOutput::new(fx.path("out.las")))
            .unwrap();
        assert_eq!(summary.outputs[0].point_format, 0);
    }
    for version in LasVersion::ALL.into_iter().filter(|v| v.supports_color()) {
        let out = fx.path(&format!("{version}.las"));
        Converter::new()
            .las_version(version)
            .convert(&fx.input, FileOutput::new(&out))
            .unwrap();
        let (header, points) = read_las(&out);
        assert_eq!(header.version().minor, version.minor());
        assert_eq!(points.len(), 5);
    }
}

#[test]
fn intensity_only_clouds_get_no_invented_colors() {
    let fx = fixture(&[cloud(vec![
        p(0.0, 0.0, 0.0).intensity(0),
        p(1.0, 0.0, 0.0).intensity(65535),
    ])]);
    let out = fx.path("out.las");
    let summary = e57_to_las::convert(&fx.input, &out).unwrap();
    assert_eq!(summary.outputs[0].point_format, 0);
    let (header, points) = read_las(&out);
    assert!(!header.point_format().has_color);
    let mut intensities: Vec<u16> = points.iter().map(|p| p.intensity).collect();
    intensities.sort_unstable();
    assert_eq!(intensities, [0, 65535]);
}

#[test]
fn gps_time_is_absolute_when_the_scan_start_is_known() {
    let start = 1.4e9;
    let fx = fixture(&[cloud(vec![
        p(0.0, 0.0, 0.0).time(0.5),
        p(1.0, 0.0, 0.0).time(2.25),
    ])
    .started_at(start)]);
    let out = fx.path("out.las");
    let summary = e57_to_las::convert(&fx.input, &out).unwrap();
    assert!(summary.warnings.is_empty(), "{:?}", summary.warnings);
    let (header, points) = read_las(&out);
    assert_eq!(header.gps_time_type(), las::GpsTimeType::Standard);
    let mut times: Vec<f64> = points.iter().map(|p| p.gps_time.unwrap()).collect();
    times.sort_by(f64::total_cmp);
    assert_eq!(times, [start + 0.5 - 1e9, start + 2.25 - 1e9]);

    let relative = fixture(&[cloud(vec![p(0.0, 0.0, 0.0).time(3.0)])]);
    let summary = e57_to_las::convert(&relative.input, relative.path("out.las")).unwrap();
    assert_eq!(summary.warnings, [Warning::RelativeTime { cloud: 0 }]);
    let (_, points) = read_las(&relative.path("out.las"));
    assert_eq!(points[0].gps_time, Some(3.0));
}

#[test]
fn returns_are_one_based_and_clamped() {
    let fx = fixture(&[cloud(vec![
        p(0.0, 0.0, 0.0).returns(0, 2),
        p(1.0, 0.0, 0.0).returns(1, 2),
        p(2.0, 0.0, 0.0).returns(8, 9),
    ])]);
    let out = fx.path("out.las");
    let summary = Converter::new()
        .threads(1)
        .stable_order(true)
        .convert(&fx.input, FileOutput::new(&out))
        .unwrap();
    assert_eq!(
        summary.warnings,
        [Warning::ReturnsClamped { cloud: 0, count: 1 }]
    );
    let (_, points) = read_las(&out);
    let returns: Vec<_> = points
        .iter()
        .map(|p| (p.return_number, p.number_of_returns))
        .collect();
    assert_eq!(returns, [(1, 2), (2, 2), (7, 7)]);
}

#[test]
fn normals_and_grid_indices_become_extra_bytes() {
    let half = std::f64::consts::FRAC_1_SQRT_2;
    let fx = fixture(&[cloud(vec![
        p(0.0, 0.0, 0.0).normal([1.0, 0.0, 0.0]).row_column(4, 7),
        p(1.0, 0.0, 0.0).normal([0.0, 0.0, 1.0]).row_column(5, 8),
    ])
    // A quarter turn around Z rotates normals too.
    .rotated([half, 0.0, 0.0, half])]);
    let out = fx.path("out.las");

    let plain = e57_to_las::convert(&fx.input, &out).unwrap();
    assert_eq!(
        read_las(&out).0.point_format().extra_bytes,
        0,
        "off by default: {plain:?}"
    );

    Converter::new()
        .threads(1)
        .stable_order(true)
        .attributes(Attributes::ALL)
        .convert(&fx.input, FileOutput::new(&out))
        .unwrap();
    let (header, points) = read_las(&out);
    assert_eq!(header.point_format().extra_bytes, 20);
    assert!(
        header
            .vlrs()
            .iter()
            .any(|v| v.user_id == "LASF_Spec" && v.record_id == 4)
    );
    let decode = |bytes: &[u8]| {
        let f = |i: usize| f32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
        let i = |i: usize| i32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
        ([f(0), f(4), f(8)], i(12), i(16))
    };
    let (n0, r0, c0) = decode(&points[0].extra_bytes);
    assert!(
        (n0[0] - 0.0).abs() < 1e-6 && (n0[1] - 1.0).abs() < 1e-6,
        "{n0:?}"
    );
    assert_eq!((r0, c0), (4, 7));
    let (n1, r1, c1) = decode(&points[1].extra_bytes);
    assert!((n1[2] - 1.0).abs() < 1e-6, "{n1:?}");
    assert_eq!((r1, c1), (5, 8));
    // The pose moved the second point to (0, 1, 0).
    assert_eq!(mm(&points[1]), [0, 1000, 0]);
}

const WKT: &str = r#"PROJCS["RGF93 / Lambert-93",GEOGCS["RGF93",DATUM["Reseau_Geodesique_Francais_1993",SPHEROID["GRS 1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Lambert_Conformal_Conic_2SP"],UNIT["metre",1],AUTHORITY["EPSG","2154"]]"#;

#[test]
fn wkt_crs_is_copied_to_las_1_4() {
    let fx = fixture_with(&[cloud(vec![p(0.0, 0.0, 0.0)])], Some(WKT));
    let out = fx.path("out.las");
    let summary = e57_to_las::convert(&fx.input, &out).unwrap();
    assert!(summary.warnings.is_empty());
    let (header, _) = read_las(&out);
    assert_eq!(header.get_wkt_crs_bytes(), Some(WKT.as_bytes()));

    let summary = Converter::new()
        .las_version(LasVersion::V1_2)
        .convert(&fx.input, FileOutput::new(&out))
        .unwrap();
    assert!(matches!(
        summary.warnings[..],
        [Warning::CrsNotWritten { .. }]
    ));
    assert!(read_las(&out).0.get_wkt_crs_bytes().is_none());

    let summary = Converter::new()
        .crs(Crs::None)
        .convert(&fx.input, FileOutput::new(&out))
        .unwrap();
    assert!(summary.warnings.is_empty());
    assert!(read_las(&out).0.get_wkt_crs_bytes().is_none());
}

#[test]
fn non_wkt_crs_is_reported_and_explicit_wkt_needs_1_4() {
    let fx = fixture_with(&[cloud(vec![p(0.0, 0.0, 0.0)])], Some("EPSG:2154"));
    let summary = e57_to_las::convert(&fx.input, fx.path("out.las")).unwrap();
    assert!(matches!(
        summary.warnings[..],
        [Warning::CrsNotWritten { .. }]
    ));

    let err = Converter::new()
        .las_version(LasVersion::V1_3)
        .crs(Crs::Wkt(WKT.into()))
        .convert(&fx.input, FileOutput::new(fx.path("out.las")))
        .unwrap_err();
    assert!(matches!(err, Error::CrsUnsupported { .. }));

    let out = fx.path("explicit.las");
    Converter::new()
        .crs(Crs::Wkt(WKT.into()))
        .convert(&fx.input, FileOutput::new(&out))
        .unwrap();
    assert_eq!(read_las(&out).0.get_wkt_crs_bytes(), Some(WKT.as_bytes()));
}

#[cfg(feature = "laz")]
#[test]
fn laz_extension_writes_compressed_output() {
    let fx = fixture(&two_clouds());
    let out = fx.path("out.laz");
    let summary = e57_to_las::convert(&fx.input, &out).unwrap();
    assert_eq!(summary.outputs[0].format, e57_to_las::output::Format::Laz);
    let (header, points) = read_las(&out);
    assert!(header.point_format().is_compressed);
    assert_eq!(points.len(), 5);
}

#[cfg(not(feature = "laz"))]
#[test]
fn laz_needs_the_feature() {
    let fx = fixture(&two_clouds());
    let err = e57_to_las::convert(&fx.input, fx.path("out.laz")).unwrap_err();
    assert!(matches!(err, Error::LazNotEnabled));
}

#[test]
fn memory_input_and_output() {
    let fx = fixture(&two_clouds());
    let bytes = std::fs::read(&fx.input).unwrap();
    let output = MemoryOutput::new();
    Converter::new()
        .grouping(Grouping::PerCloud)
        .convert(Input::bytes(bytes), &output)
        .unwrap();
    let files = output.into_files();
    assert_eq!(files.len(), 2);
    let (_, points) = read_las_bytes(files[&GroupKey::from(0)].clone());
    assert_eq!(points.len(), 3);
    assert_eq!(fx.files(), ["input.e57"]);
}

#[test]
fn custom_grouping_select_and_map_points() {
    let fx = fixture(&[
        cloud(vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0)]).named("a_1"),
        cloud(vec![p(2.0, 0.0, 0.0)]).named("a_2"),
        cloud(vec![p(3.0, 0.0, 0.0)]).named("b_1"),
        cloud(vec![p(4.0, 0.0, 0.0)]).named("skip"),
    ]);
    let summary = Converter::new()
        .select(|c| c.name.as_deref() != Some("skip"))
        .grouping(Grouping::by(|c| {
            c.name
                .as_deref()
                .unwrap_or("")
                .split('_')
                .next()
                .unwrap_or("")
                .to_owned()
        }))
        .map_points(|point| {
            point.z += 10.0;
            point.x != 1.0
        })
        .convert(&fx.input, FileOutput::new(fx.path("{key}.las")))
        .unwrap();
    assert_eq!(fx.files(), ["a.las", "b.las", "input.e57"]);
    assert_eq!(summary.points_filtered(), 1);
    assert_eq!(summary.points_written(), 3);
    assert_eq!(
        sorted_mm(&read_las(&fx.path("a.las")).1),
        [[0, 0, 10_000], [2000, 0, 10_000]]
    );
    let a = &summary.outputs[0];
    assert_eq!(a.clouds.iter().map(|c| c.index).collect::<Vec<_>>(), [0, 1]);
}

#[test]
fn header_hook_and_sensor_model() {
    let fx = fixture(&[cloud(vec![p(0.0, 0.0, 0.0)]).model("RTC360")]);
    let out = fx.path("out.las");
    e57_to_las::convert(&fx.input, &out).unwrap();
    assert_eq!(read_las(&out).0.system_identifier(), "RTC360");

    Converter::new()
        .header(|target, header| {
            header.system_identifier = "custom".into();
            header
                .vlrs
                .push(Vlr::new("MYAPP", 7, "key", target.key().to_string()));
        })
        .convert(&fx.input, FileOutput::new(&out))
        .unwrap();
    let (header, _) = read_las(&out);
    assert_eq!(header.system_identifier(), "custom");
    let vlr = header.vlrs().iter().find(|v| v.user_id == "MYAPP").unwrap();
    assert_eq!(
        (vlr.record_id, vlr.data.as_slice()),
        (7, b"merged".as_slice())
    );

    let err = Converter::new()
        .header(|_, header| header.generating_software = "x".repeat(40))
        .convert(&fx.input, FileOutput::new(&out))
        .unwrap_err();
    assert!(matches!(err, Error::InvalidOption(_)));
}

#[test]
fn colliding_outputs_fail_before_writing() {
    let fx = fixture(&two_clouds());
    let err = Converter::new()
        .grouping(Grouping::PerCloud)
        .convert(&fx.input, FileOutput::new(fx.path("same.las")))
        .unwrap_err();
    assert!(matches!(err, Error::OutputCollision { .. }), "{err:?}");
    assert_eq!(fx.files(), ["input.e57"]);
}

#[test]
fn cancelling_removes_started_outputs() {
    let clouds: Vec<Cloud> = (0..3)
        .map(|c| {
            cloud(
                (0..100_000)
                    .map(|i| p(i as f64 * 0.001, c as f64, 0.0))
                    .collect(),
            )
        })
        .collect();
    let fx = fixture(&clouds);
    let token = CancelToken::new();
    let trigger = token.clone();
    let err = Converter::new()
        .grouping(Grouping::PerCloud)
        .threads(2)
        .cancel_token(token)
        .observer(move |event| {
            if let Event::Progress { .. } = event {
                trigger.cancel();
            }
        })
        .convert(&fx.input, FileOutput::new(fx.path("out/{index}.las")))
        .unwrap_err();
    assert!(matches!(err, Error::Cancelled), "{err:?}");
    let leftovers: Vec<_> = fx
        .files()
        .into_iter()
        .filter(|f| f.ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[test]
fn a_failing_writer_aborts_the_output() {
    use std::io::{Cursor, Seek, SeekFrom, Write};

    #[derive(Debug)]
    struct Failing(Cursor<Vec<u8>>);
    impl Write for Failing {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if self.0.get_ref().len() > 10_000 {
                return Err(std::io::Error::other("disk full"));
            }
            self.0.write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl Seek for Failing {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            self.0.seek(pos)
        }
    }

    struct Tracked(Mutex<Vec<String>>);
    impl e57_to_las::output::Output for Tracked {
        type Writer = Failing;
        fn create(&self, target: &e57_to_las::output::Target<'_>) -> std::io::Result<Failing> {
            self.0
                .lock()
                .unwrap()
                .push(format!("create {}", target.key()));
            Ok(Failing(Cursor::new(Vec::new())))
        }
        fn finish(
            &self,
            target: &e57_to_las::output::Target<'_>,
            _: Failing,
        ) -> std::io::Result<()> {
            self.0
                .lock()
                .unwrap()
                .push(format!("finish {}", target.key()));
            Ok(())
        }
        fn abort(&self, target: &e57_to_las::output::Target<'_>) {
            self.0
                .lock()
                .unwrap()
                .push(format!("abort {}", target.key()));
        }
    }

    let fx = fixture(&[cloud(
        (0..200_000)
            .map(|i| p(i as f64 * 0.001, 0.0, 0.0))
            .collect(),
    )]);
    let output = Tracked(Mutex::new(Vec::new()));
    let err = Converter::new().convert(&fx.input, &output).unwrap_err();
    assert!(matches!(err, Error::Write { .. }), "{err:?}");
    assert_eq!(*output.0.lock().unwrap(), ["create merged", "abort merged"]);
}

#[test]
fn events_report_progress_to_the_total() {
    let fx = fixture(&two_clouds());
    let events = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&events);
    Converter::new()
        .observer(move |event| {
            let line = match event {
                Event::Started {
                    points_total,
                    outputs,
                    ..
                } => format!("started {points_total} {outputs}"),
                Event::Progress {
                    points_done,
                    points_total,
                } => format!("progress {points_done}/{points_total}"),
                Event::OutputFinished { output } => format!("finished {}", output.key),
                Event::CloudStarted { .. } | Event::CloudFinished { .. } => return,
                other => format!("{other:?}"),
            };
            log.lock().unwrap().push(line);
        })
        .convert(&fx.input, MemoryOutput::new())
        .unwrap();
    let events = events.lock().unwrap();
    assert_eq!(events.first().map(String::as_str), Some("started 5 1"));
    assert_eq!(events.last().map(String::as_str), Some("finished merged"));
    assert!(events.contains(&"progress 5/5".to_owned()), "{events:?}");
}

#[test]
fn empty_cloud_still_gets_a_file_and_no_selection_writes_nothing() {
    let fx = fixture(&[cloud(vec![]), cloud(vec![p(1.0, 1.0, 1.0)])]);
    let summary = Converter::new()
        .grouping(Grouping::PerCloud)
        .convert(&fx.input, FileOutput::new(fx.path("{index}.las")))
        .unwrap();
    assert_eq!(summary.points_written(), 1);
    assert_eq!(read_las(&fx.path("0.las")).1.len(), 0);

    let summary = Converter::new()
        .select(|_| false)
        .convert(&fx.input, FileOutput::new(fx.path("none.las")))
        .unwrap();
    assert!(summary.outputs.is_empty());
    assert!(!fx.path("none.las").exists());
}

#[test]
fn missing_and_invalid_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.e57");
    let err = e57_to_las::convert(&missing, dir.path().join("out.las")).unwrap_err();
    assert!(
        matches!(&err, Error::Open { path: Some(p), source } if *p == missing && source.kind() == std::io::ErrorKind::NotFound)
    );

    let garbage = dir.path().join("garbage.e57");
    std::fs::write(&garbage, b"not an e57 file at all, definitely not").unwrap();
    let err = e57_to_las::convert(&garbage, dir.path().join("out.las")).unwrap_err();
    assert!(matches!(err, Error::InvalidE57(_)), "{err:?}");
}

#[test]
fn scan_exposes_metadata_and_points() {
    let fx = fixture(&two_clouds());
    let scan = Scan::open(&fx.input).unwrap();
    assert_eq!(scan.clouds().len(), 2);
    assert_eq!(scan.point_count(), 5);
    let north = &scan.clouds()[0];
    assert_eq!(north.name.as_deref(), Some("north"));
    assert_eq!(north.pose.translation, [10.0, -20.0, 5.5]);
    assert!(north.has_color && !scan.clouds()[1].has_color);
    let bounds = north.bounds.unwrap();
    assert_eq!(bounds.min, [7.0, -22.0, 4.5]);

    let mut seen = Vec::new();
    let skipped = scan
        .for_each_point(0, Attributes::default(), |point| {
            seen.push(point.xyz());
            std::ops::ControlFlow::Continue(())
        })
        .unwrap();
    assert_eq!(skipped, 0);
    assert_eq!(seen[0], [10.0, -20.0, 5.5]);

    let summary = Converter::new()
        .convert_scan(&scan, MemoryOutput::new())
        .unwrap();
    assert_eq!(summary.points_written(), 5);
}

#[test]
fn bunny_fixture_converts() {
    let input = common::bunny();
    let dir = tempfile::tempdir().unwrap();
    let scan = Scan::open(&input).unwrap();
    for threads in [1, 0] {
        let summary = Converter::new()
            .threads(threads)
            .grouping(Grouping::PerCloud)
            .convert_scan(&scan, FileOutput::new(dir.path().join("{index}.las")))
            .unwrap();
        assert_eq!(
            summary.points_written() + summary.points_skipped(),
            scan.point_count()
        );
        for output in &summary.outputs {
            let (header, points) = read_las(output.location.as_ref().unwrap());
            assert_eq!(header.number_of_points(), output.points_written());
            assert_eq!(points.len() as u64, output.points_written());
        }
    }
}

/// Many point clouds into one LAZ file on several threads, with enough points
/// for the parallel LAZ compressor to run while other clouds are being read.
#[cfg(feature = "laz")]
#[test]
fn merged_laz_from_many_clouds_on_many_threads() {
    let clouds: Vec<Cloud> = (0..24)
        .map(|c| {
            cloud(
                (0..45_000)
                    .map(|i| {
                        p(i as f64 * 0.001, c as f64, (i % 13) as f64).color(
                            (i % 251) as u8,
                            c as u8,
                            7,
                        )
                    })
                    .collect(),
            )
        })
        .collect();
    let fx = fixture(&clouds);
    let out = fx.path("out.laz");
    let summary = Converter::new()
        .threads(4)
        .convert(&fx.input, FileOutput::new(&out))
        .unwrap();
    assert_eq!(summary.points_written(), 24 * 45_000);
    let (_, points) = read_las(&out);
    assert_eq!(points.len(), 24 * 45_000);
    assert_eq!(fx.files(), ["input.e57", "out.laz"]);
}
