use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
    time::Instant,
};

use e57_to_las::{Converter, Result};

fn main() -> Result<()> {
    let start = Instant::now();
    let input_path = "./examples/cloud.e57";
    let output_path = "./output/cloud.las";
    let converter_mt = Converter::builder().parallel().build();
    let converter_mt_split = Converter::builder().parallel().split().build();
    let converter = Converter::builder().build();

    let output_file = File::create(output_path)?;

    converter_mt.convert(PathBuf::from(&input_path), output_file)?;
    converter_mt.convert(
        move || {
            let file = File::open(input_path)?;
            Ok(BufReader::new(file))
        },
        output_path,
    )?;

    let input_buf = BufReader::new(File::open(output_path)?);

    converter.convert(input_buf, output_path)?;

    fn make_writer(id: &str) -> Result<BufWriter<File>> {
        let filename = format!("./output/{id}.las");
        let file = File::create(filename)?;

        Ok(BufWriter::new(file))
    }

    converter_mt_split.convert(input_path, |id: &str| {
        let filename = format!("./output/{id}.las");
        let file = File::create(filename)?;

        Ok(BufWriter::new(file))
    })?;

    converter_mt_split.convert(input_path, make_writer)?;

    println!("total took: {}ms", start.elapsed().as_millis());
    Ok(())
}
