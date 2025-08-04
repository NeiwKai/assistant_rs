use std::io::Cursor;
use rodio::{Decoder, OutputStream, Sink};
use reqwest;

fn speak(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching GLaDOS TTS audio...");

    // Make blocking GET request to Flask TTS server
    let response = reqwest::blocking::get("http://localhost:8124/synthesize/I%20am%20still%20alive")?;
    let audio_bytes = response.bytes()?;  // Get the WAV bytes

    // Set up audio output stream
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    // Decode the audio from memory and play it
    let cursor = Cursor::new(audio_bytes);
    let source = Decoder::new(cursor)?;
    sink.append(source);

    println!("Playing...");
    sink.sleep_until_end();

    Ok(())
}
