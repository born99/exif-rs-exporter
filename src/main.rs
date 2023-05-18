use crate::data::Error;
use clap::{Arg, Command};
use func::{export_json, parsing_metadata, read_text_lines};
use std::{
    env,
    path::{Path, PathBuf},
};

mod data;
mod func;

fn main() -> Result<(), Error> {
    let matches = Command::new("export_metadata")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
        .arg_required_else_help(true)
        .args(&[
            Arg::new("file")
                .long("file")
                .short('f')
                .help("Export image metadata from text file")
                .required(false)
                .takes_value(true),
            Arg::new("image")
                .long("image")
                .short('i')
                .help("Export metadata from specific image")
                .required(false)
                .max_values(3) //allowed for 3 file at once
                .takes_value(true),
        ])
        .get_matches();

    //should be absolute path
    let arg_text_file = matches.value_of("file");
    if arg_text_file.is_some() {
        let result = handle_arg_text_file(arg_text_file.unwrap());
        if result.is_err() {
            return Err(result.unwrap_err());
        }
    }

    // should be absolute path
    // if image is in "/test" folder => OVERWRITE
    let arg_image_path_vec = matches.get_many::<String>("image");
    if arg_image_path_vec.is_some() {
        let arg_image_path_vec = arg_image_path_vec
            .unwrap()
            .map(|f| f.to_owned())
            .collect::<Vec<_>>();

        let _ = handle_image_files(arg_image_path_vec);
    }
    Ok(())
}

pub fn handle_arg_text_file(arg_text_file: &str) -> Result<Vec<String>, Error> {
    if arg_text_file.trim().is_empty() {
        return Err(Error::EmptyArgument);
    }

    let mut successful_images = vec![];

    let cwd: PathBuf = env::current_dir().unwrap();
    let directory_path = Path::new(cwd.to_str().unwrap()).join("data");

    let _ = match read_text_lines(arg_text_file) {
        Ok(lines) => {
            for line in lines {
                if let Ok(full_image_name) = line {
                    let status =
                        extract_exif_metadata_from_image(&directory_path, &full_image_name);
                    if status.is_ok() {
                        successful_images.push(full_image_name);
                    }
                }
            }
        }
        Err(err) => {
            return Err(Error::IOError);
        }
    };

    Ok(successful_images)
}

pub fn handle_image_files(arg_image_path_vec: Vec<String>) -> Result<Vec<String>, Error> {
    let mut success_image = vec![];

    for abs_image_path in arg_image_path_vec.into_iter() {
        if abs_image_path.is_empty() {
            eprintln!("WARN: An image path is empty string! SKIPPED.");
            continue;
        }
        let abs_image_path = PathBuf::from(abs_image_path);

        let image_directory_path = abs_image_path.parent();
        let full_image_name = abs_image_path.file_name();

        if (full_image_name.is_none()) || (image_directory_path.is_none()) {
            eprintln!(
                "WARN: Something's wrong with the image path: \"{}\". SKIPPED!",
                abs_image_path.to_string_lossy()
            );
            continue;
        }

        let image_directory_path = image_directory_path.unwrap();

        let full_image_name = full_image_name.unwrap().to_str().unwrap();

        if let Ok(_) = extract_exif_metadata_from_image(image_directory_path, full_image_name) {
            success_image.push(full_image_name.to_owned());
        }
    }
    Ok(success_image)
}

pub fn extract_exif_metadata_from_image<P>(
    directory_path: P,
    full_image_name: &str,
) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    println!("INFO:  File processing: {}", full_image_name);
    if full_image_name.is_empty() {
        eprintln!("ERROR: Empty name\n");
        return Err(Error::EmptyString);
    }

    let image_path = directory_path.as_ref().join(full_image_name);

    let file = std::fs::File::open(image_path);
    if file.is_err() {
        eprintln!("WARN:  Can not open file: {}\n", full_image_name);
        return Err(Error::IOError);
    }
    let file = file.unwrap();

    let mut bufreader = std::io::BufReader::new(&file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader);
    if exif.is_err() {
        eprintln!("ERROR: Can not read exif from image: {}\n", full_image_name);
        return Err(Error::ExifMetadataError);
    }
    let exif = exif.unwrap();

    let full_img_name_vec = full_image_name.split(".").collect::<Vec<_>>();
    let image_name = full_img_name_vec[0]; //vs image_file_extension

    let image_size = match file.metadata() {
        Ok(metadata) => Some(metadata.len()),
        Err(e) => {
            eprintln!(
                "WARN: Can not get image {} size due to metadata error: {}",
                full_image_name, e
            );
            None
        }
    };

    let mut json = parsing_metadata(exif);
    json["file_name"] = serde_json::json!(&image_name);
    json["size"] = serde_json::json!(image_size);

    let status = export_json(directory_path, image_name, json);
    if status.is_err() {
        return Err(status.unwrap_err());
    }

    println!("OK:    File succeed {}\n", full_image_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        data::Error, extract_exif_metadata_from_image, handle_arg_text_file, handle_image_files,
    };
    use serde_json::Value;
    use std::{io::BufReader, path::Path};

    #[test]
    fn test_extract_exif_metadata_from_image() {
        let directory_path = Path::new("/home/born99/Woka/export_metadata/data");
        let full_image_name = "JAM19896.jpg";
        let _ = extract_exif_metadata_from_image(directory_path, full_image_name);

        // open file
        let file = std::fs::File::open("/home/born99/Woka/export_metadata/data/JAM19896.json")
            .unwrap_or_else(|e| {
                eprintln!("Can not open file {} with {}", full_image_name, e);
                panic!("Can not open file");
            });

        let reader = BufReader::new(file);
        let json_file = serde_json::from_reader::<_, Value>(reader).unwrap();

        let json_model = serde_json::json!({
            "camera_model": "Canon EOS 5D Mark IV",
            "capture_time": "2019-07-26 13:25:33",
            "created_time": "2019-07-26 13:25:33",
            "file_name": "JAM19896",
            "modified_time": "2020-08-14 12:04:00",
            "orientation": 1,
            "serial_number": "025021000537",
            "size": 953458
        });

        assert_eq!(json_file, json_model);

        // assert_eq!()
    }

    #[test]
    fn test_handle_image_file() {
        let case = vec!["/home/born99/Woka/export_metadata/data/JAM19896.jpg".to_string()];
        assert_eq!(
            handle_image_files(case),
            Ok(vec!["JAM19896.jpg".to_string()])
        );

        // empty path
        let case = vec!["".to_string()];
        assert_eq!(handle_image_files(case), Ok(vec![]));

        // blank path
        let case = vec!["    ".to_string()];
        assert_eq!(handle_image_files(case), Ok(vec![]));

        // extra dash
        let case = vec!["/JAM19896.jpg".to_string()];
        assert_eq!(handle_image_files(case), Ok(vec![]));

        // relative_path
        let case = vec!["JAM19896.jpg".to_string()];
        assert_eq!(handle_image_files(case), Ok(vec![]));

        // no such image
        let case = vec!["abcdef43256.jpg".to_string()];
        assert_eq!(handle_image_files(case), Ok(vec![]));

        // wrong file extension
        let case = vec!["JAM19896.json".to_string()];
        assert_eq!(handle_image_files(case), Ok(vec![]));
    }

    #[test]
    fn test_handle_arg_text_file() {
        // TRUE
        let case = "/home/born99/Woka/export_metadata/text.txt";
        assert_eq!(
            handle_arg_text_file(case),
            Ok(vec![
                "JAM19896.jpg".to_string(),
                "rotated_CCW90.jpg".to_string(),
                "test1.jpg".to_string(),
                "Canon_40D.jpg".to_string()
            ])
        );

        // empty path
        let case = "";
        assert_eq!(handle_arg_text_file(case), Err(Error::EmptyArgument));

        // wrong path
        let case = "/home/born99/Woka/export_metadata/data/text.txt";
        let status = handle_arg_text_file(case);
        assert_eq!(status, Err(Error::IOError));

        // blank path (no suck text file)
        let case = "    ";
        let status = handle_arg_text_file(case);
        assert_eq!(status, Err(Error::EmptyArgument));

        // extra dash path
        let case = "/home/born99/Woka/export_metadata/data/text.txt/";
        let status = handle_arg_text_file(case);
        assert_eq!(status, Err(Error::IOError));

        // empty txt
        let case = "/home/born99/Woka/export_metadata/text_empty.txt";
        let status = handle_arg_text_file(case);
        assert_eq!(status, Ok(vec![]));

        // blank txt
        let case = "/home/born99/Woka/export_metadata/text_blank.txt";
        assert_eq!(handle_arg_text_file(case), Ok(vec![]));

        // wrong file in txt
        let case = "/home/born99/Woka/export_metadata/text_wrong.txt";
        assert_eq!(handle_arg_text_file(case), Ok(vec![]));

        // extra dash txt
        let case = "/home/born99/Woka/export_metadata/text_extra_dash.txt";
        assert_eq!(handle_arg_text_file(case), Ok(vec![]));

        // absolute_path in txt
        let case = "/home/born99/Woka/export_metadata/text_abs_path.txt";
        assert_eq!(handle_arg_text_file(case), Ok(vec![]));

        // wrong file extension
        let case = "/home/born99/Woka/export_metadata/text_file_ext.txt";
        assert_eq!(handle_arg_text_file(case), Ok(vec![]));
    }
}
