use gtk::prelude::*;

#[derive(Debug, Default)]
pub struct TextHighlighter {
    buffer: gtk::TextBuffer,
    highlight_tag: gtk::TextTag,
}

impl TextHighlighter {
    pub fn new(buffer: gtk::TextBuffer) -> Self {
        let highlight_tag = buffer
            .create_tag(Some("highlight"), &[("background", &"yellow")])
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
