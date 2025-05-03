use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

async fn download_file(url: &str, output_path: &Path) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();

    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(format!("Failed to download file: HTTP status {}", response.status()).into());
    }

    let content = response.bytes().await?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(output_path, content)?;

    Ok(())
}

fn get_home_dir() -> PathBuf {
    env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| {
        if let Some(home_dir) = env::var_os("USERPROFILE") {
            PathBuf::from(home_dir)
        } else {
            PathBuf::from(".")
        }
    })
}

fn create_directory(path: &Path) -> Result<(), Box<dyn Error>> {
    if !path.exists() {
        fs::create_dir_all(path)?;
        println!("Created directory: {}", path.display());
    } else {
        println!("Directory already exists: {}", path.display());
    }
    Ok(())
}

fn move_file(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    if source.exists() {
        if let Some(parent) = destination.parent() {
            create_directory(parent)?;
        }

        fs::copy(source, destination)?;
        fs::remove_file(source)?;
        println!(
            "Moved file from {} to {}",
            source.display(),
            destination.display()
        );
    } else {
        return Err(format!("Source file does not exist: {}", source.display()).into());
    }

    Ok(())
}

fn compile_schemas(schemas_dir: &Path) -> Result<(), Box<dyn Error>> {
    let output = Command::new("glib-compile-schemas")
        .arg(schemas_dir.to_str().unwrap())
        .output()?;

    if output.status.success() {
        println!("Successfully compiled schemas");
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to compile schemas: {}", error_message).into());
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = "https://raw.githubusercontent.com/kopecmaciej/fox-reader/refs/heads/master/resources/com.github.kopecmaciej.Settings.gschema.xml";

    let temp_file = PathBuf::from("com.github.kopecmaciej.Settings.gschema.xml");

    println!("Downloading schema file from: {}", url);
    download_file(url, &temp_file).await?;

    let home_dir = get_home_dir();
    let schemas_dir = home_dir.join(".local/share/glib-2.0/schemas");
    let dest_file = schemas_dir.join("com.github.kopecmaciej.Settings.gschema.xml");

    create_directory(&schemas_dir)?;

    move_file(&temp_file, &dest_file)?;

    println!("Compiling schemas in: {}", schemas_dir.display());
    compile_schemas(&schemas_dir)?;

    println!("Installation completed successfully!");

    Ok(())
}
