
mod downloader;
mod hf;
mod config;

fn main() {
    let downloaded = Downloader {
        link: LINK.to_string(),
        save_path: "./test.onnx".to_string(),
    };

    match downloaded.download_file() {
        Ok(_) => println!("File downloaded"),
        Err(_) => panic!("Oh no"),
    };
}
