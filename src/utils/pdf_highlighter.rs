use crate::utils::highlighter::ReadingBlock;
use poppler::{Page, Rectangle};
use std::cell::RefCell;

#[derive(Debug, Clone)]
pub struct PdfReadingBlock {
    pub text: String,
    pub rectangle: Rectangle,
    pub id: u32,
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

    pub fn is_pdf_page_empty(&self, page: Page) -> bool {
        let page_text = page.text().unwrap_or_default();
        page_text.trim().is_empty()
    }

    pub fn generate_reading_blocks(&self, page: Page) {
        let mut pdf_blocks = Vec::new();
        let page_text = page.text().unwrap_or_default();

        // Skip processing if page is empty
        if page_text.trim().is_empty() {
            self.current_blocks.replace(Some(pdf_blocks));
            return;
        }

        // Parse the page text into semantic blocks
        let text_blocks = self.parse_text_into_blocks(&page_text);

        // Process each text block
        for (block_id, block) in text_blocks.into_iter().enumerate() {
            if let Some(rectangle) = self.find_block_boundaries(&page, &block) {
                let pdf_block = PdfReadingBlock {
                    text: block,
                    rectangle,
                    id: block_id as u32,
                };
                pdf_blocks.push(pdf_block);
            }
        }

        self.current_blocks.replace(Some(pdf_blocks));
    }

    /// Parses text into logical reading blocks (titles, paragraphs, sentences)
    fn parse_text_into_blocks(&self, text: &str) -> Vec<String> {
        let mut blocks = Vec::new();

        // Split into paragraphs first
        let paragraphs: Vec<&str> = text.split('\n').filter(|p| !p.trim().is_empty()).collect();

        for paragraph in paragraphs {
            // Check if paragraph is likely a title (short and ends without punctuation)
            if self.is_likely_title(paragraph) {
                blocks.push(paragraph.trim().to_string());
                continue;
            }

            // Split paragraphs into sentences
            let sentences = self.split_into_sentences(paragraph);
            blocks.extend(sentences);
        }

        blocks
    }

    /// Determines if a text block is likely a title
    fn is_likely_title(&self, text: &str) -> bool {
        let trimmed = text.trim();
        let words = trimmed.split_whitespace().count();

        // Titles are typically short and don't end with typical sentence punctuation
        words <= 12 && !trimmed.ends_with('.') && !trimmed.ends_with('?') && !trimmed.ends_with('!')
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

    /// Finds the bounding rectangle for a text block
    fn find_block_boundaries(&self, page: &Page, text_block: &str) -> Option<Rectangle> {
        // For a sentence/paragraph, we need to find the first and last words
        let words: Vec<&str> = text_block.split_whitespace().collect();
        if words.is_empty() {
            return None;
        }

        let first_word = words.first()?;
        let last_word = words.last()?;

        // Find rectangles for first and last words
        let first_rect = page.find_text(first_word).first().cloned()?;
        let last_rect = page.find_text(last_word).first().cloned()?;

        // Create a rectangle that encompasses both the first and last words
        let mut rectangle = Rectangle::new();
        rectangle.set_x1(first_rect.x1().min(last_rect.x1()));
        rectangle.set_y1(first_rect.y1().min(last_rect.y1()));
        rectangle.set_x2(last_rect.x2().max(first_rect.x2()));
        rectangle.set_y2(last_rect.y2().max(first_rect.y2()));
        Some(rectangle)
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
