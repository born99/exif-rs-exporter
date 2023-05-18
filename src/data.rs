use std::io;

use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
	#[error("File I/O error!")]
	IOError,
    #[error("Empty string found!")]
    EmptyString,
    #[error("Argument value is empty!")]
    EmptyArgument,
    #[error("Can not read exif metadata!")]
    ExifMetadataError,
}