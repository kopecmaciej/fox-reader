use pdfium_render::prelude::{
    PdfPage, PdfPageObject, PdfPageObjectCommon, PdfPageObjectsCommon, PdfPoints, PdfRect,
};

use crate::utils::highlighter::ReadingBlock;
use std::{cell::RefCell, error::Error};

#[derive(Debug, Clone)]
pub struct PdfReadingBlock {
    pub text: String,
    pub rectangles: Vec<PdfRect>,
    pub id: u32,
    pub font_family: String,
    pub font_size: f32,
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
    highlight_color: gtk::gdk::RGBA,
    current_blocks: RefCell<Vec<PdfReadingBlock>>,
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
            highlight_color: initial_rgba,
            current_blocks: RefCell::new(Vec::new()),
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
        // Filter out objects that are not valid text objects.
        let valid_text_objects: Vec<_> = page
            .objects()
            .iter()
            .filter(|object| Self::is_valid_text_object(object))
            .collect();

        let num_objects = valid_text_objects.len();
        let mut current_index = 0;
        let mut reading_blocks = Vec::new();

        let vertical_threshold = PdfPoints::new(10.0); // Example: 2 points vertically
        let horizontal_threshold = PdfPoints::new(30.0); // Example: 5 points horizontally

        while current_index < num_objects {
            let object = &valid_text_objects[current_index];
            if let Some(text_obj) = object.as_text_object() {
                if let Ok(bounds) = text_obj.bounds() {
                    let cleaned_text = text_obj
                        .text()
                        .trim()
                        .trim_start_matches(|c: char| !c.is_alphanumeric())
                        .to_string();
                    let font_size = text_obj.unscaled_font_size().value;
                    let font_family = text_obj.font().family().to_string();
                    let rect =
                        PdfRect::new(bounds.bottom(), bounds.left(), bounds.top(), bounds.right());

                    let mut current_block = PdfReadingBlock {
                        text: cleaned_text,
                        rectangles: vec![rect],
                        id: current_index as u32,
                        font_family,
                        font_size,
                    };

                    while current_index + 1 < num_objects {
                        let next_index = current_index + 1;
                        if let Some(next_text_obj) = valid_text_objects[next_index].as_text_object()
                        {
                            if let Ok(next_bounds) = next_text_obj.bounds() {
                                let next_font_size = next_text_obj.unscaled_font_size().value;
                                let same_font_size =
                                    (current_block.font_size - next_font_size).abs() < f32::EPSILON;

                                if let Some(current) = current_block.rectangles.last() {
                                    // Calculate vertical distance between text objects
                                    let vertical_distance =
                                        (current.bottom() - next_bounds.top()).abs();

                                    // Calculate horizontal distance for objects on the same line
                                    // (This is simplified - you might need to consider right-to-left text)
                                    let horizontal_distance =
                                        (current.right() - next_bounds.left()).abs();
                                    let should_merge = same_font_size
                                        && (vertical_distance <= vertical_threshold
                                            || (vertical_distance.value == 0.0
                                                && horizontal_distance <= horizontal_threshold));

                                    if should_merge {
                                        // Append text from the next object.
                                        current_block
                                            .text
                                            .push_str(&format!(" {}", next_text_obj.text()));
                                        let next_rect = PdfRect::new(
                                            next_bounds.bottom(),
                                            next_bounds.left(),
                                            next_bounds.top(),
                                            next_bounds.right(),
                                        );
                                        current_block.rectangles.push(next_rect);
                                        // Move to the next index.
                                        current_index = next_index;
                                        continue;
                                    }
                                };
                            }
                        }
                        break;
                    }

                    reading_blocks.push(current_block);
                }
            }
            current_index += 1;
        }

        if reading_blocks.is_empty() {
            return Err(String::from("Empty reading blocks").into());
        }

        self.current_blocks.replace(reading_blocks);
        Ok(())
    }

    /// Checks if the provided PDF object is a valid text object.
    /// A valid text object must have non-empty text (not only numbers or special characters)
    /// and a non-zero scaled font size.
    fn is_valid_text_object(object: &PdfPageObject) -> bool {
        if let Some(text_obj) = object.as_text_object() {
            let text = text_obj.text().to_string();
            let font_size = text_obj.scaled_font_size();
            !(text
                .trim()
                .chars()
                .all(|c| c.is_numeric() || !c.is_alphanumeric())
                || font_size.value == 0.0)
        } else {
            false
        }
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

    // Determines if a period marks the end of a sentence
    // Check for emails, titles (like Mr. Dr. etc), float numbers,
    // and any other text form that includes `.` but is not the end of the sentence
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

    pub fn get_reading_blocks(&self) -> Vec<PdfReadingBlock> {
        self.current_blocks.borrow().to_vec()
    }

    pub fn clear(&self) {
        let mut blocks = self.highlighted_blocks.borrow_mut();
        blocks.clear();
        // TODO:clear pdf highlighter area
    }
}
