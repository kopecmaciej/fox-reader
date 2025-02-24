use crate::utils::highlighter::ReadingBlock;
use gtk::prelude::*;
use poppler::Rectangle;
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub struct PdfBlock {
    pub text: String,
    pub rectangles: Vec<Rectangle>,
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
    highlighted_blocks: Arc<Mutex<Vec<PdfBlock>>>,
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
            highlighted_blocks: Arc::new(Mutex::new(Vec::new())),
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

    //fn setup_drawing(&self) {
    //    let highlight_color = self.highlight_color.clone();
    //    let highlighted_blocks = self.highlighted_blocks.clone();
    //    let document = self.document.clone();
    //    let current_page = self.current_page;
    //
    //    self.pdf_view.connect_draw(move |_, cr| {
    //        let doc = match document.lock() {
    //            Ok(doc) => doc,
    //            Err(_) => return gtk::Inhibit(false),
    //        };
    //
    //        if let Some(page) = doc.page(current_page) {
    //            // Render the PDF page first
    //            let (width, height) = page.size();
    //            let scale = self.pdf_view.allocated_width() as f64 / width;
    //
    //            cr.scale(scale, scale);
    //            page.render(cr);
    //
    //            // Now draw the highlights
    //            cr.set_source_rgba(
    //                highlight_color.red(),
    //                highlight_color.green(),
    //                highlight_color.blue(),
    //                highlight_color.alpha(),
    //            );
    //
    //            if let Ok(blocks) = highlighted_blocks.lock() {
    //                for block in blocks.iter().filter(|b| b.page_number == current_page) {
    //                    for rect in &block.rectangles {
    //                        cr.rectangle(
    //                            rect.x1(),
    //                            rect.y1(),
    //                            rect.x2() - rect.x1(),
    //                            rect.y2() - rect.y1(),
    //                        );
    //                    }
    //                }
    //                cr.fill();
    //            }
    //        }
    //    });
    //}
    //
    //pub fn extract_text_from_page(&self, page_num: i32) -> Result<String, Box<dyn Error>> {
    //    let doc = self
    //        .document
    //        .lock()
    //        .map_err(|e| format!("Failed to lock document: {}", e))?;
    //
    //    if page_num >= 0 && page_num < doc.n_pages() {
    //        if let Some(page) = doc.page(page_num) {
    //            if let Some(text) = page.text() {
    //                return Ok(text);
    //            }
    //        }
    //    }
    //
    //    Err(format!("Could not extract text from page {}", page_num).into())
    //}
    //
    //fn extract_blocks_from_page(&self, page_num: i32) -> Result<Vec<PdfBlock>, Box<dyn Error>> {
    //    let doc = self
    //        .document
    //        .lock()
    //        .map_err(|e| format!("Failed to lock document: {}", e))?;
    //
    //    if page_num >= 0 && page_num < doc.n_pages() {
    //        if let Some(page) = doc.page(page_num) {
    //            // Get text layout from the page
    //            let text = page.text();
    //            let layout = page.text_layout().unwrap_or_default();
    //
    //            // Group text by paragraphs
    //            let paragraphs = text
    //                .split('\n')
    //                .filter(|p| !p.trim().is_empty())
    //                .collect::<Vec<_>>();
    //
    //            let mut blocks = Vec::new();
    //
    //            // Process each paragraph
    //            for (p_idx, paragraph) in paragraphs.iter().enumerate() {
    //                // Split into sentences or smaller chunks
    //                let sentences = self.split_text_into_sentences(paragraph);
    //
    //                for (s_idx, sentence) in sentences.iter().enumerate() {
    //                    // Find rectangles for this text
    //                    let sentence_rects =
    //                        self.find_rectangles_for_text(&layout, sentence, &text);
    //
    //                    if !sentence_rects.is_empty() {
    //                        let block_id = format!("pdf_{}_p{}_s{}", page_num, p_idx, s_idx);
    //
    //                        blocks.push(PdfBlock {
    //                            text: sentence.clone(),
    //                            rectangles: sentence_rects,
    //                            page_number: page_num,
    //                            id: block_id,
    //                        });
    //                    }
    //                }
    //            }
    //
    //            return Ok(blocks);
    //        }
    //    }
    //
    //    Err(format!("Could not extract blocks from page {}", page_num).into())
    //}
    //
    //fn find_rectangles_for_text(
    //    &self,
    //    layout: &[poppler::TextLayoutItem],
    //    text: &str,
    //    full_text: &str,
    //) -> Vec<Rectangle> {
    //    // Find where this text appears in the full text
    //    if let Some(start_idx) = full_text.find(text) {
    //        let end_idx = start_idx + text.len();
    //
    //        // Find rectangles that overlap with this text range
    //        layout
    //            .iter()
    //            .filter(|item| {
    //                let char_idx = item.index as usize;
    //                char_idx >= start_idx && char_idx < end_idx
    //            })
    //            .map(|item| item.area)
    //            .collect()
    //    } else {
    //        Vec::new()
    //    }
    //}
    //
    //fn split_text_into_sentences(&self, text: &str) -> Vec<String> {
    //    let re = regex::Regex::new(r"([.!?])(\s+[A-Z])").unwrap();
    //    let mut raw_blocks = Vec::new();
    //    let mut start = 0;
    //
    //    for matches in re.find_iter(text) {
    //        let end = matches.start() + 1;
    //        let block = &text[start..end];
    //        if !block.is_empty() {
    //            raw_blocks.push(block.to_string());
    //        }
    //        start = matches.start() + 1;
    //    }
    //
    //    if start < text.len() {
    //        raw_blocks.push(text[start..].to_string());
    //    }
    //
    //    Self::combine_strings(raw_blocks, self.min_block_len)
    //}
    //
    //fn combine_strings(strings: Vec<String>, min_length: usize) -> Vec<String> {
    //    if strings.len() <= 1 {
    //        return strings;
    //    }
    //    let mut result = Vec::new();
    //    let mut current = String::new();
    //
    //    for (i, s) in strings.iter().enumerate() {
    //        current.push_str(s);
    //        if (current.len() >= min_length || i == strings.len() - 1) && !current.is_empty() {
    //            result.push(current);
    //            current = "".to_string();
    //        }
    //    }
    //
    //    result
    //}
    //
    ///// Highlight a specific block
    //pub fn highlight_block(&self, block: &PdfBlock) {
    //    if let Ok(mut blocks) = self.highlighted_blocks.lock() {
    //        blocks.clear();
    //        blocks.push(block.clone());
    //
    //        // Set current page to the page containing this block
    //        if self.current_page != block.page_number {
    //            let _ = self.set_current_page(block.page_number);
    //        } else {
    //            self.pdf_view.queue_draw();
    //        }
    //    }
    //}
}
