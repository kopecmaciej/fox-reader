use reqwest::blocking::{get, Response};
use std::error::Error;
use std::fs::{remove_file, File};
use std::io::copy;

pub struct FileHandler {}

impl FileHandler {
    pub fn download_file(link: String) -> Result<Response, Box<dyn Error>> {
        let response = get(link)?;
        Ok(response)
    }

    pub fn save_file(path: &str, mut res: Response) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(path)?;
        copy(&mut res, &mut file)?;
        Ok(())
    }

    pub fn remove_file(path: &str) -> Result<(), Box<dyn Error>> {
        remove_file(path)?;
        Ok(())
    }
}
