use hf::HuggingFace;

mod config;
mod downloader;
mod hf;

fn main() {
    let hf = HuggingFace::new();
    hf.get_avaliable_voices();
}
