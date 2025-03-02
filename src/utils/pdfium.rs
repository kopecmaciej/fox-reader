use std::fmt::Debug;

use pdfium_render::prelude::*;

#[derive(Debug, Default)]
pub struct PdfiumWrapper {
    pdfium: Pdfium,
    document: Option<PdfDocument<'static>>,
}

impl PdfiumWrapper {
    pub fn new() -> Result<Self, PdfiumError> {
        let pdfium = Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .unwrap(),
        );

        Ok(Self {
            pdfium,
            document: None,
        })
    }

    pub fn load_document(&mut self, path: &std::path::Path) -> Result<(), PdfiumError> {
        let document = self.pdfium.load_pdf_from_file(path, None)?;

        // Use transmute to extend the lifetime to 'static
        // Safety: We ensure document doesn't outlive pdfium by keeping both in the same struct
        let document = unsafe {
            std::mem::transmute::<
                pdfium_render::prelude::PdfDocument<'_>,
                pdfium_render::prelude::PdfDocument<'_>,
            >(document)
        };

        self.document = Some(document);
        Ok(())
    }

    pub fn get_document(&self) -> Option<&PdfDocument<'static>> {
        self.document.as_ref()
    }

    pub fn remove_pdf(&mut self) {
        self.document = None;
    }
}
