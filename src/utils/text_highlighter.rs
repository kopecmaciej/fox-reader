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
    use core::panic;

    use super::*;

    fn init_gtk() {
        if gtk::init().is_err() {
            panic!("Failed to initialize gtk")
        }
    }

    #[test]
    fn test_clear() {
        init_gtk();
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
}
