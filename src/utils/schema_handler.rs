use std::error::Error;
use std::process::Command;

use crate::paths::schema_config;
use crate::utils::file_handler::FileHandler;

pub struct SchemaHandler {}

impl SchemaHandler {
    pub fn get_schemas_dir() -> String {
        schema_config::get_schemas_dir()
    }

    pub async fn download_schema(url: &str, temp_path: &str) -> Result<(), Box<dyn Error>> {
        println!("Downloading schema file from: {}", url);
        let bytes = FileHandler::fetch_file_async(url.to_string()).await?;
        FileHandler::save_bytes(temp_path, &bytes)?;
        Ok(())
    }

    pub async fn install_schema(source: &str, dest_dir: &str) -> Result<(), Box<dyn Error>> {
        FileHandler::ensure_all_paths_exists(dest_dir)?;
        println!("Created directory: {}", dest_dir);

        let source_path = std::path::Path::new(source);
        let file_name = source_path.file_name().unwrap().to_string_lossy();
        let dest_file = format!("{}/{}", dest_dir, file_name);

        if FileHandler::does_file_exist(source) {
            let bytes = std::fs::read(source)?;
            FileHandler::save_bytes(&dest_file, &bytes)?;
            FileHandler::remove_file(source)?;
            println!("Moved file from {} to {}", source, dest_file);
        } else {
            return Err(format!("Source file does not exist: {}", source).into());
        }

        Ok(())
    }

    pub fn compile_schemas(schemas_dir: &str) -> Result<(), Box<dyn Error>> {
        let output = Command::new("glib-compile-schemas")
            .arg(schemas_dir)
            .output()?;

        if output.status.success() {
            println!("Successfully compiled schemas");
        } else {
            let error_message = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to compile schemas: {}", error_message).into());
        }

        Ok(())
    }

    pub fn schema_exists(schema_url: &str, schemas_dir: &str) -> bool {
        let file_name = schema_url.split('/').last().unwrap_or("schema.gschema.xml");
        let schema_path = format!("{}/{}", schemas_dir, file_name);

        FileHandler::does_file_exist(&schema_path)
    }

    pub async fn install_from_url() -> Result<(), Box<dyn Error>> {
        let url = schema_config::get_schema_url();
        let schemas_dir = Self::get_schemas_dir();

        if Self::schema_exists(&url, &schemas_dir) {
            println!("Schema already exists. Skipping installation.");
            return Ok(());
        }

        let file_name = url.split('/').last().unwrap_or("schema.gschema.xml");
        let temp_file = file_name.to_string();

        Self::download_schema(&url, &temp_file).await?;
        Self::install_schema(&temp_file, &schemas_dir).await?;

        println!("Compiling schemas in: {}", schemas_dir);
        Self::compile_schemas(&schemas_dir)?;

        println!("Installation completed successfully!");

        Ok(())
    }
}
