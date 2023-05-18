use crate::data::Error;
use exif::{Exif, In, Tag};
use serde_json::Value;
use std::{
    fs::File,
    io::{self, BufRead, BufWriter, Write},
    path::Path,
};

pub fn read_text_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

pub fn export_json<P>(directory_path: P, image_name: &str, json: Value) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    let full_json_file_name = format!("{}{}", image_name, ".json");
    let json_file_path = directory_path.as_ref().join(&full_json_file_name);

    let file = File::create(json_file_path);
    if file.is_err() {
        eprintln!("WARN: Can not create file: {}", &full_json_file_name);
        return Err(Error::IOError);
    }
    let file = file.unwrap();

    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, &json).unwrap();
    writer.flush().unwrap();

    Ok(())
}

pub fn parsing_metadata(exif: Exif) -> Value {
    let camera_model = match exif.get_field(Tag::Model, In::PRIMARY) {
        Some(camera_model) => Some(
            camera_model
                .display_value()
                .with_unit(&exif)
                .to_string()
                .replace("\"", ""),
        ),
        None => None,
    };

    let serial_number = match exif.get_field(Tag::BodySerialNumber, In::PRIMARY) {
        Some(serial_number) => Some(
            serial_number
                .display_value()
                .with_unit(&exif)
                .to_string()
                .replace("\"", ""),
        ),
        None => None,
    };

    // TAG description: time that img_spec are created
    let created_time = match exif.get_field(Tag::DateTimeDigitized, In::PRIMARY) {
        Some(created_time) => Some(created_time.display_value().with_unit(&exif).to_string()),
        None => None,
    };

    // TAG description: time that change file
    let modified_time = match exif.get_field(Tag::DateTime, In::PRIMARY) {
        Some(modified_time) => Some(modified_time.display_value().with_unit(&exif).to_string()),
        None => None,
    };

    let orientation = match exif.get_field(Tag::Orientation, In::PRIMARY) {
        Some(orientation) => orientation.value.get_uint(0),
        None => None,
    };

    // TAG description: capture time
    let capture_time = match exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
        Some(capture_time) => Some(capture_time.display_value().with_unit(&exif).to_string()),
        None => None,
    };

    serde_json::json!({
        "camera_model": camera_model,
        "serial_number": serial_number,
        "created_time": created_time,
        "modified_time": modified_time,
        "orientation": orientation,
        "capture_time": capture_time
    })
}
