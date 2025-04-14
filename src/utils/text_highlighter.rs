use std::{borrow::Borrow, collections::BTreeMap};

use gtk::prelude::*;

use super::highlighter::ReadingBlock;

const HIGHLIGHTED_TAG: &str = "highlighted";

#[derive(Debug, Clone)]
pub struct TextReadingBlock {
    pub id: u32,
    pub text: String,
    pub start_offset: i32,
    pub end_offset: i32,
}

impl ReadingBlock for TextReadingBlock {
    fn get_text(&self) -> String {
        self.text.clone()
    }

    fn get_id(&self) -> u32 {
        self.id
    }
}

#[derive(Debug, Default)]
pub struct TextHighlighter {
    buffer: gtk::TextBuffer,
    highlight_tag: gtk::TextTag,
    min_block_len: i32,
    current_blocks: Vec<TextReadingBlock>,
}

impl TextHighlighter {
    pub fn new(buffer: gtk::TextBuffer, min_block_len: i32, initial_color: gtk::gdk::RGBA) -> Self {
        let highlight_tag = buffer
            .create_tag(Some(HIGHLIGHTED_TAG), &[])
            .expect("Failed to create tag");

        highlight_tag.set_background_rgba(Some(&initial_color));

        Self {
            buffer,
            highlight_tag,
            min_block_len,
            current_blocks: Vec::new(),
        }
    }

    pub fn set_highlight_color(&mut self, rgba: gtk::gdk::RGBA) {
        self.highlight_tag.set_background_rgba(Some(&rgba));
    }

    pub fn is_buffer_empty(&self) -> bool {
        let buffer = self.buffer.clone();
        let (start, end) = buffer.bounds();
        let text = buffer.text(&start, &end, false);
        text.trim().is_empty()
    }

    pub fn get_text(&self) -> String {
        let buffer = self.buffer.clone();
        buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .to_string()
    }

    pub fn normalize_text(&mut self) -> String {
        let buffer = &self.buffer;

        let text = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .to_string();

        let cleaned_text = text
            .split("\n\n")
            .map(|s| s.trim())
            .collect::<Vec<&str>>()
            .join("\n\n");

        buffer.set_text(&cleaned_text);

        cleaned_text
    }

    pub fn generate_reading_blocks(&mut self) {
        let mut reading_blocks = Vec::new();
        let full_text = self.get_text();
        let blocks = self.segment_text_blocks();

        let mut current_pos = 0;
        let mut text_iter = full_text.chars().peekable();

        for (n, block) in blocks.iter().enumerate() {
            let block_chars: Vec<char> = block.chars().collect();
            let mut block_index = 0;
            let mut block_start = None;

            for c in text_iter.by_ref() {
                if block_start.is_none() && c == block_chars[block_index] {
                    block_start = Some(current_pos);
                    block_index += 1;
                } else if block_start.is_some() && c == block_chars[block_index] {
                    block_index += 1;
                } else if block_start.is_some() {
                    block_start = None;
                    block_index = 0;
                    if c == block_chars[0] {
                        block_start = Some(current_pos);
                        block_index = 1;
                    }
                }

                current_pos += 1;

                if block_index == block_chars.len() {
                    let start = block_start.unwrap();
                    reading_blocks.push(TextReadingBlock {
                        id: n as u32,
                        text: block.clone(),
                        start_offset: start,
                        end_offset: current_pos,
                    });
                    break;
                }
            }
        }

        self.current_blocks = reading_blocks;
    }

    pub fn segment_text_blocks(&self) -> Vec<String> {
        let text = self.get_text();

        let paragraphs: Vec<String> = text
            .split("\n\n")
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let mut all_blocks = Vec::new();
        for paragraph in paragraphs {
            let sentences = self.segment_sentences(paragraph);
            all_blocks.extend(sentences);
        }

        all_blocks
    }

    pub fn segment_sentences<T: Borrow<str>>(&self, text: T) -> Vec<String> {
        let re = regex::Regex::new(r"([.!?])(\s+[A-Z])").unwrap();
        let mut raw_blocks = Vec::new();
        let mut start = 0;
        let text = text.borrow();

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

        Self::combine_strings(raw_blocks, self.min_block_len as usize)
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

    pub fn update_reading_blocks(&mut self, reading_block: Vec<TextReadingBlock>) {
        self.current_blocks = reading_block;
    }

    pub fn get_reading_blocks_map(&self) -> BTreeMap<u32, TextReadingBlock> {
        let blocks = self.get_reading_blocks();
        let btree_map: BTreeMap<u32, TextReadingBlock> =
            blocks.into_iter().map(|b| (b.id, b)).collect();
        btree_map
    }

    pub fn get_reading_blocks(&self) -> Vec<TextReadingBlock> {
        self.current_blocks.to_vec()
    }

    pub fn highlight(&self, block_id: u32) {
        let blocks = self.get_reading_blocks();
        if let Some(block) = blocks.iter().find(|b| b.id == block_id) {
            self.clear();
            self.buffer.apply_tag(
                &self.highlight_tag,
                &self.buffer.iter_at_offset(block.start_offset),
                &self.buffer.iter_at_offset(block.end_offset),
            );
        }
    }

    pub fn clear(&self) {
        self.buffer.remove_tag(
            &self.highlight_tag,
            &self.buffer.start_iter(),
            &self.buffer.end_iter(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_highlighter(text: &str) -> TextHighlighter {
        let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
        buffer.set_text(text);
        TextHighlighter::new(buffer, 50, gtk::gdk::RGBA::new(255.0, 255.0, 0.0, 0.3))
    }

    #[gtk::test]
    fn test_get_text() {
        let text = "Hello, this it test sentence!";
        let highlighter = create_test_highlighter(text);

        assert_eq!(highlighter.get_text(), text);
    }

    #[gtk::test]
    fn test_clear() {
        let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
        buffer.set_text("Highlighted text");

        let highlighter =
            TextHighlighter::new(buffer, 150, gtk::gdk::RGBA::new(255.0, 255.0, 0.0, 0.3));

        highlighter.highlight(0);

        let tag = highlighter.buffer.tag_table().lookup(HIGHLIGHTED_TAG);
        assert_eq!(tag.as_ref(), Some(&highlighter.highlight_tag));

        highlighter.clear();

        let middle = highlighter.buffer.iter_at_offset(6);
        assert!(!middle.has_tag(&highlighter.highlight_tag));
        assert_eq!(middle.tags(), Vec::<gtk::TextTag>::new());
    }

    #[gtk::test]
    fn test_split_text_into_sentences() {
        let text = "First sentence. Second sentence! Third sentence? Fourth sentence.";
        let highlighter = create_test_highlighter("");

        let sentences = highlighter.segment_sentences(text);
        assert!(!sentences.is_empty());
        assert!(sentences.len() == 1);
    }

    #[gtk::test]
    fn test_split_text_into_sentences_multi_sentence() {
        let text = "First sentence. Second sentence! Third sentence? Fourth sentence.";
        let mut highlighter = create_test_highlighter("");
        highlighter.min_block_len = 7;
        let sentences = highlighter.segment_sentences(text);

        assert_eq!(sentences.len(), 4);
        assert!(sentences[0].contains("First sentence"));
        assert!(sentences[1].contains("Second sentence"));
        assert!(sentences[2].contains("Third sentence"));
        assert!(sentences[3].contains("Fourth sentence"));
    }

    #[gtk::test]
    fn test_split_text_into_sentences_edge_cases() {
        let highlighter = create_test_highlighter("");

        assert!(highlighter.segment_sentences("").is_empty());

        let single = highlighter.segment_sentences("Just one sentence.");
        assert_eq!(single.len(), 1);

        let text = "Hello!! What?! Really...".to_string();
        let multiple_punct = highlighter.segment_sentences(text);
        assert!(!multiple_punct.is_empty());
    }

    #[gtk::test]
    fn test_combine_strings() {
        let strings = vec![
            "Short sentence.".to_string(),
            "Another short one.".to_string(),
            "Third sentence.".to_string(),
        ];

        let combined = TextHighlighter::combine_strings(strings, 50);
        assert_eq!(combined.len(), 1);
    }

    #[gtk::test]
    fn test_combine_strings_edge_cases() {
        let empty: Vec<String> = vec![];
        assert!(TextHighlighter::combine_strings(empty, 10).is_empty());

        let single = vec!["One string.".to_string()];
        assert_eq!(TextHighlighter::combine_strings(single, 10).len(), 1);

        let strings = vec!["A.".to_string(), "B.".to_string(), "C.".to_string()];
        let combined = TextHighlighter::combine_strings(strings, 1);
        assert_eq!(combined.len(), 3);
    }

    #[gtk::test]
    fn test_convert_blocks_into_reading_block() {
        let text = "First sentence. Second sentence.";
        let mut highlighter = create_test_highlighter(text);

        highlighter.generate_reading_blocks();
        let blocks = highlighter.current_blocks;
        assert!(!blocks.is_empty());

        if let Some(first_block) = blocks.first() {
            assert!(first_block.start_offset >= 0);
            assert!(first_block.end_offset > first_block.start_offset);
        }
    }

    #[gtk::test]
    fn test_split_text_into_blocks() {
        let text = "First paragraph.\n\nSecond paragraph.";
        let highlighter = create_test_highlighter(text);

        let blocks = highlighter.segment_text_blocks();
        assert!(!blocks.is_empty());
    }

    #[gtk::test]
    fn test_split_text_into_blocks_with_different_separators() {
        let text = "Para 1.\n\nPara 2\n\n\nPara 3.";
        let highlighter = create_test_highlighter(text);
        let blocks = highlighter.segment_text_blocks();

        assert_eq!(blocks.len(), 3);
        assert!(blocks[0].contains("Para 1"));
        assert!(blocks[1].contains("Para 2"));
        assert!(blocks[2].contains("Para 3"));
    }

    #[gtk::test]
    fn test_split_text_into_blocks_with_whitespace() {
        let text = "   Para 1   \n\n     Para 2     \n\nPara 3   ";
        let highlighter = create_test_highlighter(text);
        let blocks = highlighter.segment_text_blocks();

        assert_eq!(blocks.len(), 3);
        assert!(blocks[0].starts_with(" "));
        assert!(blocks[0].ends_with(" "));
    }

    #[gtk::test]
    fn test_convert_blocks_into_reading_block_simple() {
        let text = "First sentence. Second sentence. Third sentence.";
        let mut highlighter = create_test_highlighter(text);
        highlighter.generate_reading_blocks();
        let blocks = highlighter.current_blocks;

        assert!(!blocks.is_empty());
        assert!(blocks.len() == 1);
        assert!(blocks[0].end_offset == 48);
    }

    #[gtk::test]
    fn test_convert_blocks_into_reading_block_complex() {
        let text = "First paragraph.\n\nSecond paragraph with multiple sentences. Another sentence here! And one more?";
        let mut highlighter = create_test_highlighter(text);
        highlighter.generate_reading_blocks();
        let blocks = highlighter.current_blocks;

        assert!(blocks.len() > 1);

        assert!(blocks[0].text.contains("First paragraph"));
        assert!(blocks[1].text.contains("Second paragraph"));
    }

    #[gtk::test]
    fn test_convert_blocks_into_reading_block_edge_cases() {
        let mut highlighter = create_test_highlighter("");
        highlighter.generate_reading_blocks();
        let empty_blocks = highlighter.current_blocks.to_vec();
        assert!(empty_blocks.is_empty());

        let text = "Hello! @#$% World?\n\nSpecial chars: &*()";
        let mut highlighter = create_test_highlighter(text);
        highlighter.generate_reading_blocks();
        let special_blocks = highlighter.current_blocks;
        assert!(!special_blocks.is_empty());
    }

    #[gtk::test]
    fn test_macbeth_text_processing() {
        let text = "Malcolm. The worthy thane of Ross.

Lennox. What a haste looks through his eyes! So should he look
That seems to speak things strange.

Ross. God save the king!

Duncan. Whence camest thou, worthy thane?

Ross. From Fife, great king;
Where the Norweyan banners flout the sky
And fan our people cold. Norway himself,
With terrible numbers,
Assisted by that most disloyal traitor
The thane of Cawdor, began a dismal conflict;
Till that Bellona's bridegroom, lapp'd in proof,
Confronted him with self-comparisons,
Point against point rebellious, arm 'gainst arm.
Curbing his lavish spirit: and, to conclude,
The victory fell on us.";

        let mut highlighter = create_test_highlighter(text);
        highlighter.min_block_len = 250;

        let blocks = highlighter.segment_text_blocks();
        assert_eq!(blocks.len(), 6);

        assert!(blocks[0].contains("Malcolm"));
        assert!(blocks[1].contains("Lennox"));
        assert!(blocks[2].contains("Ross. God save"));
        assert!(blocks[3].contains("Duncan"));
        assert!(blocks[4].contains("Ross. From Fife"));
        assert!(blocks[5].contains("Curbing his lavish spirit"));

        highlighter.generate_reading_blocks();
        let reading_blocks = highlighter.current_blocks;
        assert!(!reading_blocks.is_empty());

        assert_eq!(reading_blocks[0].start_offset, 0);
        assert_eq!(reading_blocks[5].end_offset, 628);
    }
}
