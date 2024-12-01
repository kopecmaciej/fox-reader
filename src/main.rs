use hf::HuggingFace;

mod config;
mod downloader;
mod hf;

fn main() {
    let hf = HuggingFace::new();
    match hf.parse_avaliable_voices() {
        Ok(_) => println!("It's ok"),
        Err(e) => panic!("{e}"),
    }
}
