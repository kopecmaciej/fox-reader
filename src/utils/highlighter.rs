use std::fmt;

pub trait ReadingBlock: fmt::Debug {
    fn get_text(&self) -> String;
    fn get_id(&self) -> u32;
}
