use std::{
    error::Error,
    fmt::Debug,
    fs::{self, create_dir_all},
    path::Path,
};

use flate2::bufread::GzDecoder;
use pdfium_render::prelude::*;
use tar::Archive;

use crate::{core::runtime::runtime, paths::get_pdfium_path};

#[derive(Debug, Default)]
pub struct PdfiumWrapper {
    pdfium: Option<Pdfium>,
    document: Option<PdfDocument<'static>>,
}

impl PdfiumWrapper {
    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        let pdfium_path = runtime().block_on(Self::ensure_pdfium_available())?;

        let pdfium = Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&pdfium_path))
                .or_else(|_| Pdfium::bind_to_system_library())
                .map_err(|e| format!("Failed to bind to Pdfium library: {}", e))?,
        );

        self.pdfium = Some(pdfium);
        Ok(())
    }

    async fn ensure_pdfium_available() -> Result<String, Box<dyn Error>> {
        let pdfium_dir = get_pdfium_path();
        create_dir_all(&pdfium_dir)?;

        let library_path = Path::new(&pdfium_dir).join("libpdfium.so");
        if library_path.exists() {
            return Ok(pdfium_dir);
        }

        let temp_path = Path::new("/tmp/pdfium");

        let download_url = "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7047/pdfium-linux-x64.tgz";

        let compressed_data = reqwest::get(download_url).await?.bytes().await?;

        let gz = GzDecoder::new(&compressed_data[..]);
        let mut archive = Archive::new(gz);
        archive.unpack(temp_path)?;

        let source_lib_path = temp_path.join("lib/libpdfium.so");

        if !source_lib_path.exists() {
            return Err("libpdfium.so not found in extracted archive".into());
        }

        let dest_lib_path = Path::new(&pdfium_dir).join("libpdfium.so");
        fs::copy(&source_lib_path, &dest_lib_path)?;

        let _ = fs::remove_dir(temp_path);

        Ok(pdfium_dir)
    }

    pub fn load_document(&mut self, path: &std::path::Path) -> Result<(), Box<dyn Error>> {
        if let Some(pdfium) = &self.pdfium {
            let document = pdfium.load_pdf_from_file(path, None)?;

            // Use transmute to extend the lifetime to 'static
            // Safety: We ensure document doesn't outlive pdfium by keeping both in the same struct
            let document = unsafe {
                std::mem::transmute::<
                    pdfium_render::prelude::PdfDocument<'_>,
                    pdfium_render::prelude::PdfDocument<'_>,
                >(document)
            };

            self.document = Some(document);

            return Ok(());
        }
        Err("Pdfium not initialized properly".into())
    }

    pub fn get_document(&self) -> Option<&PdfDocument<'static>> {
        self.document.as_ref()
    }

    pub fn remove_pdf(&mut self) {
        self.document = None;
    }
}
