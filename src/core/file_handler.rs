use reqwest::get as get_async;
use std::error::Error;
use std::fs::{self, remove_file, File};
use std::io::prelude::*;
use std::path::Path;

pub struct FileHandler {}

impl FileHandler {
    pub fn does_file_exist(file_path: &str) -> bool {
        Path::new(file_path).exists()
    }

    pub fn ensure_all_dirs_exists(path: &str) -> Result<(), std::io::Error> {
        let path = Path::new(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        Ok(())
    }

    pub async fn fetch_file_async(link: String) -> Result<Vec<u8>, Box<dyn Error>> {
        let response = get_async(link).await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    pub fn save_bytes(path: &str, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
        Self::ensure_all_dirs_exists(path)?;
        let mut file = File::create(path)?;
        file.write_all(bytes)?;
        Ok(())
    }

    pub fn remove_file(path: &str) -> Result<(), Box<dyn Error>> {
        remove_file(path)?;
        Ok(())
    }

    pub fn get_all_file_names(path: &str) -> Result<Vec<String>, Box<dyn Error>> {
        if !Self::does_file_exist(path) {
            return Ok(Vec::new());
        }
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

    pub fn get_env_value(
        file_path: &str,
        env_name: &str,
    ) -> Result<Option<String>, Box<dyn Error>> {
        let content = fs::read_to_string(file_path)?;
        let re = regex::Regex::new(&format!(r"{}='([^']+)'", env_name))?;
        if let Some(cap_group) = re.captures(&content) {
            if let Some(value) = cap_group.get(1) {
                return Ok(Some(value.as_str().to_owned()));
            };
        };
        Ok(None)
    }

    pub fn update_env(
        file_path: &str,
        env_name: &str,
        new_env: &str,
    ) -> Result<(), Box<dyn Error>> {
        let content = fs::read_to_string(file_path)?;
        let re = regex::Regex::new(&format!(r"{}='[^']+'", env_name))?;

        let updated = re.replace(&content, format!("{}='{}'", env_name, new_env));

        fs::write(file_path, updated.to_string())?;

        Ok(())
    }

    pub fn get_default_voice_from_config(path: &str) -> Result<Option<String>, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        Ok(content
            .lines()
            .find(|line| line.trim().contains("DefaultVoice"))
            .and_then(|line| line.split_whitespace().nth(1))
            .map(|s| s.to_string()))
    }

    pub fn delete_line_from_config(path: &str, line_to_remove: &str) -> Result<(), Box<dyn Error>> {
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
            let updated_line = format!("\n{} {}", key, new_value);
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
