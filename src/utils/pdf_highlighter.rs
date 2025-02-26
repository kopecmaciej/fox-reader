use pdfium_render::prelude::{
    PdfPage, PdfPageObjectCommon, PdfPageObjectsCommon, PdfPoints, PdfRect,
};

use crate::utils::highlighter::ReadingBlock;
use std::{cell::RefCell, error::Error};

#[derive(Debug, Clone)]
pub struct PdfReadingBlock {
    pub text: String,
    pub rectangle: PdfRect,
    pub id: u32,
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
}

impl ReadingBlock for PdfReadingBlock {
    fn get_text(&self) -> String {
        self.text.clone()
    }

    fn get_id(&self) -> u32 {
        self.id
    }
}

#[derive(Debug)]
pub struct PdfHighlighter {
    current_page: i32,
    highlight_color: gtk::gdk::RGBA,
    current_blocks: RefCell<Option<Vec<PdfReadingBlock>>>,
    highlighted_blocks: RefCell<Vec<u32>>,
}

impl Default for PdfHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfHighlighter {
    pub fn new() -> Self {
        let initial_rgba = gtk::gdk::RGBA::new(1.0, 1.0, 0.0, 0.3);

        Self {
            current_page: 0,
            highlight_color: initial_rgba,
            current_blocks: RefCell::new(None),
            highlighted_blocks: RefCell::new(Vec::new()),
        }
    }

    pub fn set_highlight_color(&mut self, rgba: gtk::gdk::RGBA) {
        self.highlight_color = rgba;
    }

    pub fn is_pdf_page_empty(&self, page: &PdfPage) -> bool {
        let page_text = page.text().unwrap();
        page_text.all().trim().is_empty()
    }

    pub fn generate_reading_blocks(&self, page: PdfPage) -> Result<(), Box<dyn Error>> {
        let mut raw_text_objects = Vec::new();

        let max_sentence_lenght = 100;

        // Step 1: Collect all valid text objects (filter out numbers and special chars)
        for (mut n, obj) in page.objects().iter().enumerate() {
            if let Some(text_obj) = obj.as_text_object() {
                if let Ok(bounds) = text_obj.bounds() {
                    let text = text_obj.text().to_string();

                    let font_size = text_obj.scaled_font_size();
                    let font_family = text_obj.font().family().to_string();
                    // Skip if text is just a number or special character
                    if text
                        .trim()
                        .chars()
                        .all(|c| c.is_numeric() || !c.is_alphanumeric())
                        || font_size.value == 0.0
                    {
                        continue;
                    }

                    let rect =
                        PdfRect::new(bounds.bottom(), bounds.left(), bounds.top(), bounds.right());

                    let mut current_block = PdfReadingBlock {
                        text,
                        rectangle: rect,
                        id: n as u32,
                        font_family: Some(font_family),
                        font_size: Some(font_size.value),
                    };

                    if let Ok(obj) = page.objects().get(n + 1) {
                        if let Some(obj) = obj.as_text_object() {
                            if obj.scaled_font_size() == font_size {
                                current_block.text.push_str(&obj.text());
                                //skip next one
                            }
                        }
                    }

                    raw_text_objects.push(current_block);
                }
            }
        }

        self.current_blocks.replace(Some(raw_text_objects));
        Ok(())
    }

    // Helper function to merge two PdfRect objects
    fn merge_rectangles(rect1: &PdfRect, rect2: &PdfRect) -> PdfRect {
        PdfRect::new(
            rect1.bottom().min(rect2.bottom()),
            rect1.left().min(rect2.left()),
            rect1.top().max(rect2.top()),
            rect2.right().max(rect2.right()),
        )
    }

    /// Splits a paragraph into sentences
    fn split_into_sentences(&self, paragraph: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current_sentence = String::new();

        // Simple sentence splitter that handles common end-of-sentence markers
        for char in paragraph.chars() {
            current_sentence.push(char);

            // Check for end of sentence
            if char == '.' || char == '!' || char == '?' {
                // Check if this is really the end of a sentence (not part of an abbreviation, etc.)
                let is_sentence_end = self.is_sentence_end(&current_sentence);

                if is_sentence_end {
                    sentences.push(current_sentence.trim().to_string());
                    current_sentence = String::new();
                }
            }
        }

        // Add any remaining text as a sentence
        if !current_sentence.trim().is_empty() {
            sentences.push(current_sentence.trim().to_string());
        }

        sentences
    }

    /// Determines if a period marks the end of a sentence
    fn is_sentence_end(&self, text: &str) -> bool {
        // Common abbreviations that shouldn't be treated as sentence boundaries
        let common_abbreviations = [
            "Mr.", "Mrs.", "Ms.", "Dr.", "Prof.", "Inc.", "Ltd.", "Co.", "e.g.", "i.e.", "etc.",
        ];

        for abbr in &common_abbreviations {
            if text.trim().ends_with(abbr) {
                return false;
            }
        }

        // Check for decimal numbers like 3.14
        if let Some(c) = text.chars().rev().nth(1) {
            if c.is_ascii_digit() {
                let mut has_letter_after = false;
                let chars: Vec<char> = text.chars().collect();
                for i in (0..chars.len()).rev() {
                    if chars[i] == '.' {
                        break;
                    }
                    if chars[i].is_alphabetic() {
                        has_letter_after = true;
                        break;
                    }
                }
                if !has_letter_after {
                    return false;
                }
            }
        }

        true
    }

    pub fn highlight(&self, block_id: u32) {
        self.clear();
        self.highlighted_blocks.borrow_mut().push(block_id);
    }

    pub fn get_reading_blocks(&self) -> Option<Vec<PdfReadingBlock>> {
        self.current_blocks.borrow().as_ref().cloned()
    }

    pub fn clear(&self) {
        let mut blocks = self.highlighted_blocks.borrow_mut();
        blocks.clear();
        // TODO:clear pdf highlighter area
    }
}
