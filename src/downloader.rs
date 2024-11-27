use reqwest::blocking::{get, Response};
use std::fs::File;
use std::io::copy;

pub struct Downloader {}

impl Downloader {
    pub fn download_file(link: String) -> Result<Response, String> {
        let response = match get(link) {
            Ok(resp) => resp,
            Err(e) => return Err(format!("Failed to get file from URL: {}", e)),
        };
        Ok(response)
    }

    pub fn save_file(save_path: String, mut res: Response) -> Result<(), String> {
        let mut file = match File::create(save_path) {
            Ok(file) => file,
            Err(e) => return Err(format!("Failed to create file: {}", e)),
        };

        match copy(&mut res, &mut file) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to copy data to file: {}", e)),
        }
    }
}
