use pdfium_render::prelude::{
    PdfPage, PdfPageObject, PdfPageObjectCommon, PdfPageObjectsCommon, PdfPageTextObject,
    PdfPoints, PdfQuadPoints, PdfRect, PdfSearchDirection, PdfSearchOptions,
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
        // 1. Filter out objects that are not valid text objects
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
            self.process_text_into_blocks(&page, &mut reading_blocks, text_obj, bounds)?;
        }

        if reading_blocks.is_empty() {
            return Err(String::from("Empty reading blocks").into());
        }

        self.current_blocks.replace(reading_blocks);
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
        page: &PdfPage,
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
        let font_family = text_obj.font().family().to_string();
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
                font_family.clone(),
            );

            let left = PdfPoints::new(right.value + char_length);
            let next_rect = PdfRect::new(bounds.bottom(), left, bounds.top(), bounds.right());

            let id = reading_blocks.last().map(|last| last.id + 1).unwrap_or(0);
            let new_block = PdfReadingBlock {
                text: next_sentence.to_string(),
                rectangles: vec![next_rect],
                id,
                font_family,
                font_size,
            };

            reading_blocks.push(new_block);
        } else {
            // Add entire text as single block or merge with previous
            self.add_text_to_blocks(reading_blocks, cleaned_text, rect, font_size, font_family);
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
        font_family: String,
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
            font_family,
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

            // Check font size and spatial positioning
            same_font_size
                && (vertical_distance.value <= VERTICAL_THRESHOLD.into()
                    || (vertical_distance.value == 0.0
                        && horizontal_distance.value <= HORIZONTAL_THRESHOLD.into()))
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

    // Determines if a period marks the end of a sentence and returns its index
    // Returns the index of the sentence-ending period, or 0 if not a sentence end
    fn find_sentence_end_index(&self, text: &str) -> usize {
        // Bail early if text doesn't contain a period
        if !text.contains('.') {
            return 0;
        }

        let trimmed = text.trim();

        // Find the last period in the text
        if let Some(last_period_pos) = trimmed.rfind('.') {
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

            // Common abbreviations that shouldn't be treated as sentence boundaries
            let common_abbreviations = [
                "Mr.", "Mrs.", "Ms.", "Dr.", "Prof.", "Inc.", "Ltd.", "Co.", "e.g.", "i.e.",
                "etc.", "vs.", "Fig.", "St.", "Ave.", "Blvd.", "Corp.", "Dept.", "Est.", "Jr.",
                "Sr.", "Ph.D.", "B.A.", "M.A.", "a.m.", "p.m.", "U.S.", "U.K.", "E.U.", "v.",
                "Jan.", "Feb.", "Mar.", "Apr.", "Jun.", "Jul.", "Aug.", "Sept.", "Oct.", "Nov.",
                "Dec.",
            ];

            // Extract the word containing the period
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            for word in words.iter().rev() {
                if word.contains('.') {
                    // Check if this word is an abbreviation
                    for abbr in &common_abbreviations {
                        if word == abbr
                            || word.ends_with(abbr)
                                && !word[..word.len() - abbr.len()]
                                    .chars()
                                    .last()
                                    .unwrap_or(' ')
                                    .is_alphanumeric()
                        {
                            return 0;
                        }
                    }
                    break;
                }
            }

            // Check for website and file extensions
            let extensions = [
                ".com", ".org", ".net", ".edu", ".gov", ".io", ".html", ".pdf", ".txt", ".rs",
                ".js",
            ];
            let last_word = trimmed.split_whitespace().last().unwrap_or("");
            for ext in &extensions {
                if last_word.to_lowercase().ends_with(ext) {
                    return 0;
                }
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

            // Check for ellipsis (...) which isn't a sentence ending
            if (last_period_pos >= 2 && &trimmed[last_period_pos - 2..=last_period_pos] == "...")
                || trimmed.contains("….")
            {
                return 0;
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

        // No period found (shouldn't happen at this point, but just in case)
        0
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
