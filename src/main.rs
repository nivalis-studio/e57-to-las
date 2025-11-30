use std::time::Instant;

use e57_to_las::{ConvertOptions, Result, convert, convert_split, parallel};

fn main() -> Result<()> {
    let start = Instant::now();
    let input_path = "./examples/Trimble_StSulpice-Cloud-50mm.e57";
    let output_path = "./outputs/Trimble_StSulpice-Cloud-50mm.las";

    let opts: ConvertOptions = Default::default();

    time_it("convert", || {
        convert(input_path, output_path, &opts).unwrap();
    });

    time_it("convert_split", || {
        convert_split(input_path, &output_path, &opts).unwrap();
    });

    time_it("parallel::convert_split", || {
        parallel::convert_split(input_path, output_path, &opts).unwrap();
    });
    time_it("parallel::convert_split", || {
        parallel::convert(&input_path, output_path, &opts).unwrap();
    });

    println!("total took: {}ms", start.elapsed().as_millis());

    Ok(())
}

fn time_it<F>(label: &str, f: F)
where
    F: FnOnce(),
{
    let start = Instant::now();

    f();

    println!("{label} took: {}ms", start.elapsed().as_millis());
}
