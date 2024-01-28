use thiserror::Error;


#[derive(Error, Debug)]
pub enum Error {
    #[error("Error getting dependency from the internet.")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Package was not a valid zip.")]
    ZipError(#[from] zip::result::ZipError),
    #[error("Zipped file has an absolute file location. This is not allowed.")]
    ZipSecurityError,
    #[error("Error reading/writing files.")]
    IoError(#[from] std::io::Error),
    #[error("Could not find Maven artifact {0}.")]
    NotFoundError(String),
    #[error("Could not search directory for C++ library objects.")]
    JwalkError(#[from] jwalk::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
