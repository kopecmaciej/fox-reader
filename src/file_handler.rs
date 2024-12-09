use reqwest::blocking::{get, Response};
use std::error::Error;
use std::fs::{self, remove_file, File};
use std::io::{copy, Read};
use std::io::prelude::*;

pub struct FileHandler {}

impl FileHandler {
    pub fn download_file(link: String) -> Result<Response, Box<dyn Error>> {
        let response = get(link)?;
        Ok(response)
    }

    pub fn save_file<R>(path: &str, data: &mut R) -> Result<(), Box<dyn Error>>
    where
        R: Read,
    {
        let mut file = File::create(path)?;
        copy(data, &mut file)?;
        Ok(())
    }

    pub fn remove_file(path: &str) -> Result<(), Box<dyn Error>> {
        remove_file(path)?;
        Ok(())
    }

    pub fn get_all_file_names(path: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let files = fs::read_dir(path)?;

        let file_names: Vec<String> = files
            .filter_map(|file| file.ok().and_then(|f| f.file_name().into_string().ok()))
            .collect();

        Ok(file_names)
    }

    pub fn append_to_file(path: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(path)?;

        if let Err(e) = file.write_all(data) {
            return Err(e.into());
        }
        Ok(())
    }
}
