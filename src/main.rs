use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
    time::Instant,
};

use e57_to_las::{Converter, Result};

fn main() -> Result<()> {
    let start = Instant::now();
    let input_path = "./examples/Trimble_StSulpice-Cloud-50mm.e57";
    let output_path = "./output/Trimble_StSulpice-Cloud-50mm.las";
    let converter = Converter::builder().build();

    let output_file = File::create(output_path)?;

    converter.convert(PathBuf::from(&input_path), output_file)?;
    converter.convert(
        move || {
            let file = File::open(input_path)?;
            Ok(BufReader::new(file))
        },
        output_path,
    )?;

    fn make_writer(id: &str) -> Result<BufWriter<File>> {
        let filename = format!("./output/{id}.las");
        let file = File::create(filename)?;

        Ok(BufWriter::new(file))
    }

    converter.convert_split_pointclouds(input_path, |id: &str| {
        let filename = format!("./output/{id}.las");
        let file = File::create(filename)?;

        Ok(BufWriter::new(file))
    })?;

    converter.convert_split_pointclouds(input_path, make_writer)?;

    println!("total took: {}ms", start.elapsed().as_millis());
    Ok(())
}
