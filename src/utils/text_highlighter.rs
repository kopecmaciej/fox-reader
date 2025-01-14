use gtk::prelude::*;

const HIGHLIGHTED_TAG: &str = "highlighted";

#[derive(Debug, Default)]
pub struct TextHighlighter {
    buffer: gtk::TextBuffer,
    highlight_tag: gtk::TextTag,
}

impl TextHighlighter {
    pub fn new(buffer: gtk::TextBuffer) -> Self {
        let highlight_tag = buffer
            .create_tag(Some(HIGHLIGHTED_TAG), &[("background", &"yellow")])
            .expect("Failed to create tag");

        Self {
            buffer,
            highlight_tag,
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

    pub fn split_text_into_reading_block(&self) -> Vec<String> {
        let text = self.get_text();

        let re = regex::Regex::new(r"([.!?])\s{1,}[A-Z]").unwrap();
        let mut raw_blocks = Vec::new();
        let mut start = 0;

        for mat in re.find_iter(&text) {
            let end = mat.end();
            let sentence = &text[start..end - 1].trim();
            if !sentence.is_empty() {
                raw_blocks.push(sentence.to_string());
            }
            start = end - 1;
        }

        if start < text.len() {
            raw_blocks.push(text[start..].trim().to_string());
        }

        let mut combined_blocks = Vec::new();
        let mut current_block = String::new();

        for block in raw_blocks {
            if current_block.len() + block.len() + 1 < 200 {
                if !current_block.is_empty() {
                    current_block.push(' ');
                }
                current_block.push_str(&block);
            } else {
                if !current_block.is_empty() {
                    combined_blocks.push(current_block);
                }
                current_block = block;
            }
        }

        if !current_block.is_empty() {
            combined_blocks.push(current_block);
        }

        combined_blocks
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

        let highlighter = TextHighlighter::new(buffer);

        highlighter.highlight(0, 11);

        let tag = highlighter.buffer.tag_table().lookup(HIGHLIGHTED_TAG);
        assert_eq!(tag.as_ref(), Some(&highlighter.highlight_tag));

        highlighter.clear();

        let middle = highlighter.buffer.iter_at_offset(6);
        assert!(!middle.has_tag(&highlighter.highlight_tag));
        assert_eq!(middle.tags(), Vec::<gtk::TextTag>::new());
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

        let highlighter = TextHighlighter::new(buffer);
        let blocks = highlighter.split_text_into_reading_block();
        assert_eq!(blocks.len(), 6);
    }
}
