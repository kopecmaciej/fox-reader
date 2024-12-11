use reqwest::blocking::{get, Response};
use std::error::Error;
use std::fs::{self, remove_file, File};
use std::io::prelude::*;
use std::io::{copy, Read};
use std::path::Path;

pub struct FileHandler {}

impl FileHandler {
    pub fn does_file_exist(file_path: &str) -> bool {
        Path::new(file_path).exists()
    }

    pub fn create_all_dirs(path: &str) -> Result<(), std::io::Error> {
        let path = Path::new(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        Ok(())
    }

    pub fn download_file(link: String) -> Result<Response, Box<dyn Error>> {
        let response = get(link)?;
        Ok(response)
    }

    pub fn save_file<R>(path: &str, data: &mut R) -> Result<(), Box<dyn Error>>
    where
        R: Read,
    {
        Self::create_all_dirs(path)?;
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
            .append(true)
            .open(path)?;

        if let Err(e) = file.write_all(data) {
            return Err(e.into());
        }
        Ok(())
    }

    pub fn remove_line_from_config(path: &str, line_to_remove: &str) -> Result<(), Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let updated_content = content
            .lines()
            .filter(|line| {
                if line.trim() == line_to_remove.trim() {
                    return false;
                }
                true
            })
            .collect::<Vec<&str>>()
            .join("\n");

        fs::write(path, updated_content)?;

        Ok(())
    }

    pub fn upsert_value_in_config(
        path: &str,
        key: &str,
        new_value: &str,
    ) -> Result<(), Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        if !content.contains(key) {
            let updated_line = format!("{} {}", key, new_value);
            Self::append_to_file(path, updated_line.as_bytes())
        } else {
            let content = content
                .lines()
                .map(|line| {
                    if line.starts_with(key) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 && parts[0] == key {
                            return format!("{} {}", key, new_value);
                        }
                    }
                    line.to_string()
                })
                .collect::<Vec<String>>()
                .join("\n");
            fs::write(path, content).map_err(|e| e.into())
        }
    }
}
