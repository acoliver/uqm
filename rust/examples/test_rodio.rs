//! Test rodio audio playback with a real OGG file

use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

fn main() {
    println!("Testing rodio audio playback...");

    // Try to open an OGG file from the content directory
    let ogg_path = "../sc2/content/addons/3domusic/starbase.ogg";

    let file = match File::open(ogg_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open {}: {}", ogg_path, e);
            eprintln!("Trying alternative path...");

            // Try another path
            match File::open("sc2/content/addons/3domusic/starbase.ogg") {
                Ok(f) => f,
                Err(e2) => {
                    eprintln!("Also failed: {}", e2);
                    return;
                }
            }
        }
    };

    println!("Opened file successfully");

    // Initialize audio output
    let (_stream, stream_handle) = match rodio::OutputStream::try_default() {
        Ok(s) => {
            println!("Audio output initialized");
            s
        }
        Err(e) => {
            eprintln!("Failed to initialize audio: {}", e);
            return;
        }
    };

    // Create a sink
    let sink = match rodio::Sink::try_new(&stream_handle) {
        Ok(s) => {
            println!("Created sink");
            s
        }
        Err(e) => {
            eprintln!("Failed to create sink: {}", e);
            return;
        }
    };

    // Decode the file
    let reader = BufReader::new(file);
    let source = match rodio::Decoder::new(reader) {
        Ok(s) => {
            println!("Decoded OGG file");
            s
        }
        Err(e) => {
            eprintln!("Failed to decode: {}", e);
            return;
        }
    };

    // Play it
    println!("Playing audio for 5 seconds...");
    sink.append(source);

    // Wait for 5 seconds
    std::thread::sleep(Duration::from_secs(5));

    println!("Done!");
}
