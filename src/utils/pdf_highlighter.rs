use pdfium_render::prelude::{
    PdfPage, PdfPageTextSegment, PdfPoints, PdfQuadPoints, PdfRect, PdfSearchDirection,
    PdfSearchOptions,
};

use crate::utils::highlighter::ReadingBlock;
use std::{collections::BTreeMap, error::Error};

#[derive(Debug, Clone)]
pub struct PdfReadingBlock {
    pub text: String,
    pub rectangles: Vec<PdfRect>,
    pub id: u32,
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
    pub current_page_num: Option<u16>,
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
            current_page_num: None,
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
        if let Some(page_num) = self.current_page_num {
            if curr_page_num != page_num {
                self.current_page_num = Some(curr_page_num)
            }
        };

        //TODO: change to page.text() as objects() don't return all the text
        let text = page.text()?;

        let mut reading_blocks: Vec<PdfReadingBlock> = Vec::new();

        for segment in text.segments().iter() {
            // Process text and create reading blocks
            self.process_text_into_blocks(&mut reading_blocks, segment)?;
        }

        if reading_blocks.is_empty() {
            return Err(String::from("Empty reading blocks").into());
        }

        Self::merge_blocks_on_same_level(reading_blocks.as_mut());
        self.current_blocks = reading_blocks;
        Ok(())
    }

    pub fn merge_blocks_on_same_level(blocks: &mut [PdfReadingBlock]) {
        // Define a tolerance for vertical alignment
        // This value may need adjustment based on your PDF's font size and layout
        const VERTICAL_TOLERANCE: f32 = 2.0;

        for block in blocks.iter_mut() {
            // We need at least 2 rectangles to merge
            if block.rectangles.len() <= 1 {
                continue;
            }

            // Sort rectangles by their vertical position (using the middle point)
            // This helps ensure we're comparing rectangles on similar lines
            block.rectangles.sort_by(|a, b| {
                let a_mid = (a.top() + a.bottom()) / 2.0;
                let b_mid = (b.top() + b.bottom()) / 2.0;
                a_mid
                    .partial_cmp(&b_mid)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let mut i = 0;
            while i < block.rectangles.len() - 1 {
                let current = &block.rectangles[i];
                let next = &block.rectangles[i + 1];

                // Calculate vertical midpoints
                let current_mid_y = (current.top() + current.bottom()) / 2.0;
                let next_mid_y = (next.top() + next.bottom()) / 2.0;

                // Check if rectangles are on approximately the same line
                // and if the next rectangle is to the right of the current one
                if (current_mid_y.value - next_mid_y.value).abs() <= VERTICAL_TOLERANCE
                    && current.right() <= next.left()
                {
                    // Create merged rectangle that encompasses both
                    // Use the min/max of coordinates to handle varying heights properly
                    let bottom = current.bottom().min(next.bottom());
                    let left = current.left();
                    let top = current.top().max(next.top());
                    let right = next.right();

                    let merged_rect = PdfRect::new(bottom, left, top, right);

                    block.rectangles[i] = merged_rect;
                    block.rectangles.remove(i + 1);

                    // Don't increment i since we need to check the newly merged rectangle
                    // against the next one
                } else {
                    i += 1;
                }
            }
        }
    }

    // Helper method to process text and add to reading blocks
    fn process_text_into_blocks(
        &self,
        reading_blocks: &mut Vec<PdfReadingBlock>,
        segment: PdfPageTextSegment,
    ) -> Result<(), Box<dyn Error>> {
        let text = segment.text();

        let bounds = segment.bounds();
        // 2. Clean the text from special characters at the beginning
        let cleaned_text = text
            .trim()
            .trim_start_matches(|c: char| !c.is_alphanumeric())
            .to_string();
        // Extract text properties

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

            self.add_text_to_blocks(reading_blocks, current_sentence.to_string(), current_rect);

            let left = PdfPoints::new(right.value + char_length);
            let next_rect = PdfRect::new(bounds.bottom(), left, bounds.top(), bounds.right());

            let id = reading_blocks.last().map(|last| last.id + 1).unwrap_or(0);
            let new_block = PdfReadingBlock {
                text: next_sentence.to_string(),
                rectangles: vec![next_rect],
                id,
            };

            reading_blocks.push(new_block);
        } else {
            // Add entire text as single block or merge with previous
            self.add_text_to_blocks(reading_blocks, cleaned_text, rect);
        }
        Ok(())
    }

    // Helper to add text to blocks
    fn add_text_to_blocks(
        &self,
        reading_blocks: &mut Vec<PdfReadingBlock>,
        text: String,
        rect: PdfRect,
    ) {
        if let Some(last_block) = reading_blocks.last_mut() {
            if self.should_merge_with_last_block(last_block, &rect) {
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
        };

        reading_blocks.push(new_block);
    }

    fn should_merge_with_last_block(
        &self,
        last_block: &PdfReadingBlock,
        current_rect: &PdfRect,
    ) -> bool {
        if let Some(last_rect) = last_block.rectangles.last() {
            let vertical_distance = (last_rect.bottom() - current_rect.top()).abs();
            let horizontal_distance = (last_rect.right() - current_rect.left()).abs();
            let last_dot = last_block.text.trim().ends_with(".");

            !last_dot
                && (vertical_distance.value <= VERTICAL_THRESHOLD.into()
                    || horizontal_distance.value <= HORIZONTAL_THRESHOLD.into())
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
}
