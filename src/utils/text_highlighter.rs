use gtk::prelude::*;

const HIGHLIGHTED_TAG: &str = "highlighted";

#[derive(Debug)]
pub struct ReadBlock {
    pub block: String,
    pub start_offset: i32,
    pub end_offset: i32,
}

#[derive(Debug, Default)]
pub struct TextHighlighter {
    buffer: gtk::TextBuffer,
    highlight_tag: gtk::TextTag,
    min_block_len: i32,
}

impl TextHighlighter {
    pub fn new(buffer: gtk::TextBuffer, min_block_len: i32) -> Self {
        let highlight_tag = buffer
            .create_tag(Some(HIGHLIGHTED_TAG), &[("background", &"yellow")])
            .expect("Failed to create tag");

        Self {
            buffer,
            highlight_tag,
            min_block_len,
        }
    }

    pub fn get_text(&self) -> String {
        let buffer = self.buffer.clone();
        buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .to_string()
            // let's replace "" as it will break the script
            .replace("\"", "'")
    }

    pub fn convert_blocks_into_reading_block(&self) -> Vec<ReadBlock> {
        let mut reading_blocks = Vec::new();

        let mut current_offset: i32 = 0;
        let blocks = self.split_text_into_block();
        for block in blocks {
            let mut start_offset = current_offset;
            let mut end_offset = current_offset + block.len() as i32;
            current_offset = end_offset;
            let trim_start = block.len() - block.trim_start().len();
            let trim_end = block.len() - block.trim_end().len();
            start_offset += trim_start as i32;
            end_offset += trim_end as i32;
            reading_blocks.push(ReadBlock {
                start_offset,
                end_offset,
                block,
            });
        }

        reading_blocks
    }

    pub fn split_text_into_block(&self) -> Vec<String> {
        let text = self.get_text();

        let re = regex::Regex::new(r"([.!?])(\s+[A-Z])").unwrap();
        let mut raw_blocks = Vec::new();
        let mut start = 0;

        for matches in re.find_iter(&text) {
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

    pub fn highlight(&self, start_offset: i32, end_offset: i32) {
        self.clear();
        self.buffer.apply_tag(
            &self.highlight_tag,
            &self.buffer.iter_at_offset(start_offset),
            &self.buffer.iter_at_offset(end_offset),
        );
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
mod test {
    use super::*;

    #[gtk::test]
    fn test_clear() {
        let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
        buffer.set_text("Highlighted text");

        let highlighter = TextHighlighter::new(buffer, 150);

        highlighter.highlight(0, 11);

        let tag = highlighter.buffer.tag_table().lookup(HIGHLIGHTED_TAG);
        assert_eq!(tag.as_ref(), Some(&highlighter.highlight_tag));

        highlighter.clear();

        let middle = highlighter.buffer.iter_at_offset(6);
        assert!(!middle.has_tag(&highlighter.highlight_tag));
        assert_eq!(middle.tags(), Vec::<gtk::TextTag>::new());
    }

    #[gtk::test]
    fn test_convert_blocks_into_reading_block_with_whitespace() {
        let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
        let test_text = "When Mr. Bilbo Baggins of Bag End announced that he would shortly be
celebrating his eleventy-first birthday with a party of special
magnificence, there was much talk and excitement in Hobbiton.
Bilbo was very rich and very peculiar, and had been the
wonder of the Shire for sixty years, ever since his remarkable
disappearance and unexpected return. The riches he had brought back
from his travels had now become a local legend, and it was popularly
believed, whatever the old folk might say, that the Hill at Bag End
was full of tunnels stuffed with treasure. And if that was not enough
for fame, there was also his prolonged vigour to marvel at. Time wore
on, but it seemed to have little effect on Mr. Baggins. At ninety he
was much the same as at fifty.";
        buffer.set_text(test_text);

        let highlighter = TextHighlighter::new(buffer, 150);
        let reading_blocks = highlighter.convert_blocks_into_reading_block();

        assert_eq!(reading_blocks.len(), 5);

        assert_eq!(reading_blocks[0].start_offset, 0);
        assert_eq!(reading_blocks[0].end_offset, 194);

        assert_eq!(reading_blocks[1].start_offset, 194);
        assert_eq!(reading_blocks[1].end_offset, 350);

        assert_eq!(reading_blocks[2].start_offset, 350);
        assert_eq!(reading_blocks[2].end_offset, 561);

        assert_eq!(reading_blocks[3].start_offset, 561);
        assert_eq!(reading_blocks[3].end_offset, 714);

        assert_eq!(reading_blocks[4].start_offset, 714);
        assert_eq!(reading_blocks[4].end_offset, 758);
    }

    #[gtk::test]
    fn test_split_text_into_reading_blocks() {
        let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
        let test_text = "When Mr. Bilbo Baggins of Bag End announced that he would shortly be
celebrating his eleventy-first birthday with a party of special
magnificence, there was much talk and excitement in Hobbiton.
Bilbo was very rich and very peculiar, and had been the
wonder of the Shire for sixty years, ever since his remarkable
disappearance and unexpected return. The riches he had brought back
from his travels had now become a local legend, and it was popularly
believed, whatever the old folk might say, that the Hill at Bag End
was full of tunnels stuffed with treasure. And if that was not enough
for fame, there was also his prolonged vigour to marvel at. Time wore
on, but it seemed to have little effect on Mr. Baggins. At ninety he
was much the same as at fifty. At ninety-nine they began to call him
well-preserved; but unchanged would have been nearer the mark. There
were some that shook their heads and thought this was too much of a
good thing; it seemed unfair that anyone should possess (apparently)
perpetual youth as well as (reputedly) inexhaustible wealth.
\"It will have to be paid for,\" they said. \"It isn\"t natural,
and trouble will come of it!";
        buffer.set_text(test_text);

        let highlighter = TextHighlighter::new(buffer, 150);
        let blocks = highlighter.split_text_into_block();
        assert_eq!(blocks.len(), 5);
    }
}
