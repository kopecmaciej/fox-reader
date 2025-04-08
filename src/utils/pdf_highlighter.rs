use pdfium_render::prelude::{
    PdfPage, PdfPageObject, PdfPageObjectCommon, PdfPageObjectsCommon, PdfPageTextObject,
    PdfPoints, PdfQuadPoints, PdfRect, PdfSearchDirection, PdfSearchOptions,
};

use crate::utils::highlighter::ReadingBlock;
use std::{collections::BTreeMap, error::Error};

#[derive(Debug, Clone)]
pub struct PdfReadingBlock {
    pub text: String,
    pub rectangles: Vec<PdfRect>,
    pub id: u32,
    pub font_size: f32,
}

// TODO: test thresholds
// Let's merge text that's near enough to make some sense of merging it
const VERTICAL_THRESHOLD: u8 = 10;
const HORIZONTAL_THRESHOLD: u8 = 40;

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
    pub blocks_generated_for: Option<u16>,
    pub current_blocks: Vec<PdfReadingBlock>,
    pub highlighted_block: Option<u32>,
}

impl Default for PdfHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfHighlighter {
    pub fn new() -> Self {
        Self {
            blocks_generated_for: None,
            current_blocks: Vec::new(),
            highlighted_block: None,
        }
    }

    pub fn is_pdf_page_empty(&self, page: &PdfPage) -> bool {
        let page_text = page.text().unwrap();
        page_text.all().trim().is_empty()
    }

    pub fn get_reading_blocks_map(&self) -> BTreeMap<u32, PdfReadingBlock> {
        let blocks = self.get_reading_blocks();
        let btree_map: BTreeMap<u32, PdfReadingBlock> =
            blocks.into_iter().map(|b| (b.id, b)).collect();
        btree_map
    }

    pub fn generate_reading_blocks(
        &mut self,
        page: &PdfPage,
        curr_page_num: u16,
    ) -> Result<(), Box<dyn Error>> {
        // 1. Filter out objects that are not valid text objects
        if let Some(page_num) = self.blocks_generated_for {
            if curr_page_num != page_num {
                self.blocks_generated_for = Some(curr_page_num)
            }
        };

        //TODO: change to page.text() as objects() don't return all the text
        let valid_text_objects: Vec<_> = page
            .objects()
            .iter()
            .filter(|object| Self::is_valid_text_object(object))
            .collect();

        let mut reading_blocks: Vec<PdfReadingBlock> = Vec::new();

        for object in valid_text_objects.iter() {
            let text_obj = match object.as_text_object() {
                Some(obj) => obj,
                None => continue,
            };

            let bounds = match text_obj.bounds() {
                Ok(bounds) => bounds,
                Err(_) => continue,
            };

            // Process text and create reading blocks
            self.process_text_into_blocks(&mut reading_blocks, text_obj, bounds)?;
        }

        if reading_blocks.is_empty() {
            return Err(String::from("Empty reading blocks").into());
        }

        self.current_blocks = reading_blocks;
        Ok(())
    }

    fn _search_for_text_rect(
        &self,
        page: &PdfPage,
        text: &str,
        bounds: &PdfQuadPoints,
    ) -> Result<Option<PdfRect>, Box<dyn Error>> {
        let search_opt = &PdfSearchOptions::new()
            .match_case(true)
            .match_whole_word(true);
        let pdf_text = page.text()?;
        let pdf_text_search = pdf_text.search(text, search_opt);
        if pdf_text_search.find_next().is_none() {
            // for some reason sometimes if sentence is move to another line hyphen is missing
            //pdf_text_search = pdf_text.search(&format!("{text}-"), search_opt);
            if pdf_text_search.find_next().is_none() {
                return Ok(None);
            }
        }
        for pdf_text_iter in pdf_text_search.iter(PdfSearchDirection::SearchForward) {
            for text_segment in pdf_text_iter.iter() {
                if text_segment.bounds().top() == bounds.top() {
                    return Ok(Some(text_segment.bounds()));
                }
            }
        }
        Ok(None)
    }

    // Helper method to process text and add to reading blocks
    fn process_text_into_blocks(
        &self,
        reading_blocks: &mut Vec<PdfReadingBlock>,
        text_obj: &PdfPageTextObject,
        bounds: PdfQuadPoints,
    ) -> Result<(), Box<dyn Error>> {
        // 2. Clean the text from special characters at the beginning
        let cleaned_text = text_obj
            .text()
            .trim()
            .trim_start_matches(|c: char| !c.is_alphanumeric())
            .to_string();
        // Extract text properties
        let font_size = text_obj.unscaled_font_size().value;
        let rect = PdfRect::new(bounds.bottom(), bounds.left(), bounds.top(), bounds.right());

        let sentence_end_index = self.find_sentence_end_index(&cleaned_text);

        if sentence_end_index > 0 {
            let char_length = bounds.width().value / cleaned_text.len() as f32;

            let (mut current_sentence, mut next_sentence) =
                cleaned_text.split_at(sentence_end_index);
            current_sentence = current_sentence.trim();
            next_sentence = next_sentence.trim();
            // Calculate character width and positions
            let move_by = char_length * current_sentence.len() as f32;

            // better precision of highlighting the text
            let right = PdfPoints::new(bounds.left().value + move_by);
            let current_rect = PdfRect::new(bounds.bottom(), bounds.left(), bounds.top(), right);

            self.add_text_to_blocks(
                reading_blocks,
                current_sentence.to_string(),
                current_rect,
                font_size,
            );

            let left = PdfPoints::new(right.value + char_length);
            let next_rect = PdfRect::new(bounds.bottom(), left, bounds.top(), bounds.right());

            let id = reading_blocks.last().map(|last| last.id + 1).unwrap_or(0);
            let new_block = PdfReadingBlock {
                text: next_sentence.to_string(),
                rectangles: vec![next_rect],
                id,
                font_size,
            };

            reading_blocks.push(new_block);
        } else {
            // Add entire text as single block or merge with previous
            self.add_text_to_blocks(reading_blocks, cleaned_text, rect, font_size);
        }
        Ok(())
    }

    // Helper to add text to blocks
    fn add_text_to_blocks(
        &self,
        reading_blocks: &mut Vec<PdfReadingBlock>,
        text: String,
        rect: PdfRect,
        font_size: f32,
    ) {
        if let Some(last_block) = reading_blocks.last_mut() {
            if self.should_merge_with_last_block(last_block, &rect, font_size) {
                // Merge with the last block
                last_block.text.push_str(&format!(" {}", text));
                last_block.rectangles.push(rect);
                return;
            }
        }

        let id = reading_blocks.last().map(|last| last.id + 1).unwrap_or(0);

        // Create a new block
        let new_block = PdfReadingBlock {
            text,
            rectangles: vec![rect],
            id,
            font_size,
        };

        reading_blocks.push(new_block);
    }

    fn should_merge_with_last_block(
        &self,
        last_block: &PdfReadingBlock,
        current_rect: &PdfRect,
        current_font_size: f32,
    ) -> bool {
        let last_font_size = last_block.font_size;
        let same_font_size = (current_font_size - last_font_size).abs() < f32::EPSILON;

        if let Some(last_rect) = last_block.rectangles.last() {
            let vertical_distance = (last_rect.bottom() - current_rect.top()).abs();
            let horizontal_distance = (last_rect.right() - current_rect.left()).abs();
            let last_dot = last_block.text.trim().ends_with(".");

            // Check font size and spatial positioning
            same_font_size
                && !last_dot
                && (vertical_distance.value <= VERTICAL_THRESHOLD.into()
                    || horizontal_distance.value <= HORIZONTAL_THRESHOLD.into())
        } else {
            false
        }
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

    // TODO: Rewrite this so more cases could be easly added
    fn find_sentence_end_index(&self, text: &str) -> usize {
        if !text.contains('.') {
            return 0;
        }

        let trimmed = text.trim();

        // Find the last period in the text
        if let Some(last_period_pos) = trimmed.rfind('.') {
            if last_period_pos == trimmed.len() - 1 {
                return 0;
            }
            // Get the absolute position in the original text
            let original_pos = text.len() - (trimmed.len() - last_period_pos);

            // Extract the context around the period for analysis
            let before_period = &trimmed[..last_period_pos];
            let after_period = &trimmed[last_period_pos + 1..];

            // Check for email addresses
            if trimmed.contains('@') {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if let Some(last_word) = parts.last() {
                    if last_word.contains('@') && last_word.contains('.') {
                        return 0;
                    }
                }
            }

            let first_word = trimmed.split_whitespace().last().unwrap_or("");
            if first_word.contains("http") {
                return 0;
            }

            // Check for decimal numbers
            let numbers_pattern = last_period_pos > 0
                && before_period
                    .chars()
                    .rev()
                    .take(1)
                    .all(|c| c.is_ascii_digit())
                && after_period.chars().all(|c| c.is_ascii_digit());
            if numbers_pattern {
                return 0;
            }

            // Check for version numbers like "1.0.2"
            if trimmed.matches('.').count() > 1 {
                let parts: Vec<&str> = trimmed.split('.').collect();
                if parts
                    .iter()
                    .all(|part| part.chars().all(|c| c.is_ascii_digit()))
                {
                    return 0;
                }
            }

            // Check for quoted text that ends with a period inside the quotes
            if after_period.is_empty()
                && (before_period.ends_with('\"') || before_period.ends_with('\''))
                && ((before_period.matches('"').count() % 2 == 1)
                    || (before_period.matches('\'').count() % 2 == 1))
            {
                // The period is likely part of the quotation, not a sentence end
                return 0;
            }

            // If we've passed all these checks, it's likely a sentence ending
            return original_pos + 1;
        }

        0
    }

    pub fn highlight(&mut self, block_id: u32) {
        self.clear_highlight();
        self.highlighted_block = Some(block_id);
    }

    pub fn get_reading_blocks(&self) -> Vec<PdfReadingBlock> {
        self.current_blocks.to_vec()
    }

    pub fn get_highlighted_block(&self) -> Option<PdfReadingBlock> {
        if let Some(highlighted) = self.highlighted_block {
            return self
                .get_reading_blocks()
                .into_iter()
                .find(|b| b.id == highlighted);
        }
        None
    }

    pub fn clear_highlight(&mut self) {
        self.highlighted_block = None;
    }
}
