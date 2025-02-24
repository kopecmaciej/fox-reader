use crate::utils::highlighter::ReadingBlock;
use gtk::prelude::*;
use poppler::Rectangle;
use std::{
    cell::RefCell,
    error::Error,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub struct PdfBlock {
    pub text: String,
    pub rectangle: Rectangle,
    pub id: u32,
}

impl ReadingBlock for PdfBlock {
    fn get_text(&self) -> String {
        self.text.clone()
    }

    fn get_id(&self) -> u32 {
        self.id
    }
}

/// Manages highlighting within a PDF document
pub struct PdfHighlighter {
    document: Arc<Mutex<poppler::Document>>,
    current_page: i32,
    highlight_color: gtk::gdk::RGBA,
    current_blocks: RefCell<Option<Vec<PdfBlock>>>,
    highlighted_blocks: RefCell<Vec<u32>>,
    pdf_view: gtk::DrawingArea,
    min_block_len: usize,
}

impl PdfHighlighter {
    pub fn new(
        document: poppler::Document,
        pdf_view: gtk::DrawingArea,
        min_block_len: usize,
    ) -> Self {
        let initial_rgba = gtk::gdk::RGBA::new(1.0, 1.0, 0.0, 0.3);

        let highlighter = Self {
            document: Arc::new(Mutex::new(document)),
            current_page: 0,
            highlight_color: initial_rgba,
            current_blocks: RefCell::new(None),
            highlighted_blocks: RefCell::new(Vec::new()),
            pdf_view,
            min_block_len,
        };

        //highlighter.setup_drawing();

        highlighter
    }

    pub fn set_highlight_color(&mut self, rgba: gtk::gdk::RGBA) {
        self.highlight_color = rgba;
        // Redraw to update color
        self.pdf_view.queue_draw();
    }

    fn extract_blocks_from_page(&self, page_num: i32) -> Result<Vec<PdfBlock>, Box<dyn Error>> {
        let doc = self
            .document
            .lock()
            .map_err(|e| format!("Failed to lock document: {}", e))?;

        let mut pdf_blocks = Vec::new();
        if let Some(page) = doc.page(page_num) {
            let page_text = page.text().unwrap_or_default();
            for (n, word) in page_text.split_whitespace().enumerate() {
                if let Some(rect) = page.find_text(word).first() {
                    let pdf_block = PdfBlock {
                        text: word.into(),
                        rectangle: *rect,
                        id: n as u32,
                    };
                    pdf_blocks.push(pdf_block);
                }
            }
        };

        Ok(pdf_blocks)
    }

    fn split_blocks_into_sentences(&self, text: &str) -> Vec<String> {
        let re = regex::Regex::new(r"([.!?])(\s+[A-Z])").unwrap();
        let mut raw_blocks = Vec::new();
        let mut start = 0;

        for matches in re.find_iter(text) {
            let end = matches.start() + 1;
            let block = &text[start..end];
            if !block.is_empty() {
                raw_blocks.push(block.to_string());
            }
            start = matches.start() + 1;
        }

        if start < text.len() {
            raw_blocks.push(text[start..].to_string());
        }

        Self::combine_strings(raw_blocks, self.min_block_len)
    }

    fn combine_strings(strings: Vec<String>, min_length: usize) -> Vec<String> {
        if strings.len() <= 1 {
            return strings;
        }
        let mut result = Vec::new();
        let mut current = String::new();

        for (i, s) in strings.iter().enumerate() {
            current.push_str(s);
            if (current.len() >= min_length || i == strings.len() - 1) && !current.is_empty() {
                result.push(current);
                current = "".to_string();
            }
        }

        result
    }

    pub fn highlight_block(&self, block: &PdfBlock) {
        let mut blocks = self.highlighted_blocks.borrow_mut();
        blocks.clear();
        blocks.push(block.id);

        self.pdf_view.queue_draw();
    }
}
