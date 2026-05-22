// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate rand;
use base64::{decode, encode};
use chrono::Utc;
use rand::rngs::OsRng;
use rand::RngCore;
use serde_json::json;
use serde_json::Value;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![generate_otp_key, encrypt, decrypt])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn generate_otp_key(
    file_path: String,
    file_size: u64,
    window: tauri::Window,
) -> Result<String, String> {
    println!("Received file path: {}", file_path);

    std::thread::spawn(move || {
        let otp_size: u64 = file_size * 1024 * 1024; // For a 1GB key
                                                     //let otp_size: u64 = file_size;

        // Open the file for writing
        let file = match File::create(&file_path) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Failed to create file: {}", e); // Log the error
                return; // Exit the closure early
            }
        };

        let mut writer = BufWriter::new(file);

        // Create a buffer for the random data
        let mut buffer = vec![0u8; 10 * 1024 * 1024]; // 1MB buffer

        // Calculate the number of full buffers to write
        let full_chunks = otp_size / buffer.len() as u64;

        // Write the full chunks
        for i in 0..full_chunks {
            // rand::thread_rng().fill_bytes(&mut buffer);
            OsRng.fill_bytes(&mut buffer);

            if let Err(e) = writer.write_all(&buffer) {
                eprintln!("Failed to write to file: {}", e);
                return;
            }

            // Emit progress event
            if i % 2 == 0 {
                let progress = ((i as f64 + 1.0) / full_chunks as f64) * 100.0;
                print!("\rProgress: {}%", progress.round());
                std::io::stdout().flush().unwrap();
                // Make sure to handle potential errors from emitting events, for example using `expect` or proper error handling
                window
                    .emit("otp-key-generation-progress", progress)
                    .expect("Failed to emit progress");
            }
        }

        // Write any remaining bytes that don't fit in a full buffer
        let remaining_bytes = (otp_size % buffer.len() as u64) as usize;
        if remaining_bytes > 0 {
            buffer.resize(remaining_bytes, 0);
            // rand::thread_rng().fill_bytes(&mut buffer);
            OsRng.fill_bytes(&mut buffer);
            // match writer.write_all(&buffer) {
            //     Ok(_) => {} // If successful, do nothing
            //     Err(e) => {
            //         eprintln!("Failed to write to file: {}", e); // Log the error
            //         return; // Exit the closure early
            //     }
            // };
            if let Err(e) = writer.write_all(&buffer) {
                eprintln!("Failed to write to file: {}", e);
                return;
            }
        }

        match writer.flush() {
            Ok(_) => {} // If successful, do nothing
            Err(e) => {
                eprintln!("Failed to flush writer: {}", e); // Log the error
                return; // Exit the closure early
            }
        };

        // Append metadata after successfully writing the key
        if let Err(e) = append_metadata_to_key(&file_path, &None, otp_size, &None) {
            eprintln!("Failed to append metadata: {}", e);
            return;
        }

        // Once done, you can emit an event to the frontend indicating completion
        window
            .emit("otp-key-generation-complete", {})
            .expect("Failed to emit event");
    });

    Ok("Key generation started".into())
}

#[tauri::command]
fn encrypt(plaintext_msg: String, file_path: String) -> Result<String, String> {
    println!("\n--encrypt-- \n");
    let file = match File::open(&file_path) {
        Ok(file) => file,
        Err(e) => return Err(e.to_string()),
    };

    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    let mut encrypted_msg = Vec::new();
    let mut plaintext_bytes = plaintext_msg.into_bytes().into_iter();

    // Read the file
    reader.read_to_end(&mut buffer).map_err(|e| e.to_string())?;

    let separator = "\n---METADATA---\n".as_bytes(); // Convert separator to byte slice
                                                     // Search for the separator in the buffer
    let separator_index = buffer
        .windows(separator.len())
        .position(|window| window == separator);

    // println!("buffer: {}", buffer[0]);

    // Step 1:  Get key buffer
    let key_buffer = match separator_index {
        Some(index) => &buffer[0..index],
        None => &buffer,
    };

    // println!("key_buffer: {}", key_buffer[0]);

    // Step 2:  Get metadata_str
    let metadata_bytes = match separator_index {
        Some(index) => &buffer[index + separator.len()..],
        None => &[], // Return an empty byte slice if the separator is not found
    };

    // Convert the byte slice to a string, using lossy conversion for potentially invalid UTF-8 data
    let metadata_cow = String::from_utf8_lossy(metadata_bytes);
    let metadata_str: &str = &metadata_cow;

    // println!("Metadata Content: {:?}", metadata_str);
    // println!("buffer_length {:?}", buffer.len());

    // Step 3:  Parse Metadata

    let mut generation_date: Option<String> = None;
    let mut filename: Option<String> = None;

    match serde_json::from_str::<Value>(metadata_str) {
        Ok(metadata) => {
            // Extract generationDate
            if let Some(date_str) = metadata["generationDate"].as_str() {
                generation_date = Some(date_str.to_string());
            } else {
                println!("Warning: 'generationDate' not found in metadata.");
            }

            // Extract filename
            if let Some(file_str) = metadata["filename"].as_str() {
                filename = Some(file_str.to_string());
            } else {
                println!("Warning: 'filename' not found in metadata.");
            }
        }
        Err(e) => println!("Error parsing metadata: {}", e),
    }

    // Step 4:  Encrypt the data up to the metadata
    for (i, &byte) in key_buffer.iter().enumerate() {
        if let Some(msg_byte) = plaintext_bytes.next() {
            // Debugging print statements
            // println!("Iteration: {}", i);
            // println!("Key byte ({}): {:?} [char: '{}']", i, byte, byte as char);
            // println!(
            //     "Message byte ({}): {:?} [char: '{}']",
            //     i, msg_byte, msg_byte as char
            // );
            // println!(
            //     "Resulting byte: {:?} [char: '{}']",
            //     msg_byte ^ byte,
            //     (msg_byte ^ byte) as char
            // );

            encrypted_msg.push(msg_byte ^ byte);
        } else {
            // Update the file by removing the used part of the key and updating the metadata
            // Skip the current byte as it's already used
            let new_key_content = match separator_index {
                Some(index) => &buffer[0..index], // does not use up the key
                // Some(index) => &buffer[i + 1..index], // Use the found separator index
                None => &buffer[i + 1..], // Default to the rest of the buffer if separator is not found
            };

            // Open the file in write mode to update it
            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&file_path)
                .map_err(|e| e.to_string())?;

            // println!("\nnew_key_content: {}", encode(new_key_content));

            // Write the remaining key content back to the file
            file.write_all(new_key_content).map_err(|e| e.to_string())?;

            file.flush().map_err(|e| e.to_string())?;

            // Append updated metadata
            append_metadata_to_key(
                &file_path,
                &filename,
                new_key_content.len() as u64,
                &generation_date,
            )
            .map_err(|e| e.to_string())?;

            append_metadata_to_msg(
                &mut encrypted_msg,
                &file_path,
                &filename,
                new_key_content.len(),
                generation_date,
            );

            break; // If the plaintext message is shorter than the key, break out of the loop
        }
    }

    // println!("encrypted_msg : {:?}", encrypted_msg);

    // Convert the encrypted message to a Base64 String and return it
    Ok(encode(&encrypted_msg))
}

#[tauri::command]
fn decrypt(encrypted_msg: String, file_path: String) -> Result<String, String> {
    println!("\n--decrypt--\n");

    // Step 1: Base64 Decode
    let encrypted_msg_bytes = decode(&encrypted_msg).map_err(|e| e.to_string())?;

    // println!("encrypted_msg_bytes : {:?}", encrypted_msg_bytes);

    // Step 2: Find the Separator and Split
    let separator = b"-->>"; // Byte string for the separator
    let separator_len = separator.len();
    let position = encrypted_msg_bytes // Find the position of the separator
        .windows(separator_len)
        .position(|window| window == separator);

    // println!("position : {:?}", position);

    let (payload, metadata_with_separator) = match position {
        // Split the byte slice at the position of the separator
        Some(pos) => encrypted_msg_bytes.split_at(pos),
        None => return Err("Separator not found".into()),
    };

    // Adjust for the separator to get only the metadata part
    let metadata_bytes = if metadata_with_separator.len() > separator_len {
        &metadata_with_separator[separator_len..]
    } else {
        return Err("No metadata after separator".into());
    };

    // println!("payload : {:?}", String::from_utf8_lossy(&payload));
    //println!(
    //     "metadata_bytes: {:?}",
    //     String::from_utf8_lossy(&metadata_bytes)
    // );

    // Step 3: Extract and Parse Metadata
    // Parse metadata assuming it's JSON
    let metadata_str = std::str::from_utf8(metadata_bytes)
        .map_err(|e| e.to_string())?
        .trim_start_matches(char::from(separator[0]));
    let metadata: Value = serde_json::from_str(metadata_str).map_err(|e| e.to_string())?;

    if let Some(generation_date) = metadata["generationDate"].as_str() {
        println!("Generation Date: {}", generation_date);
    } else {
        return Err("Missing generationDate in metadata".into());
    }

    // Step 4: Process the Payload

    // Open the key file and read its content
    let mut key_file_content = Vec::new();

    File::open(&file_path)
        .map_err(|e| e.to_string())?
        .read_to_end(&mut key_file_content)
        .map_err(|e| e.to_string())?;

    // Decrypt the message using the key bytes
    let mut decrypted_msg = Vec::new();

    for (i, &enc_byte) in payload.iter().enumerate() {
        let key_byte = key_file_content
            .get(i % key_file_content.len())
            .ok_or("Key byte index out of range")?;
        decrypted_msg.push(enc_byte ^ key_byte);

        // println!("enc_byte: {:?}", enc_byte);
        // println!("key_byte: {:?}", key_byte);
    }

    println!(
        "decrypted_msg {:?}",
        String::from_utf8_lossy(&decrypted_msg)
    );

    // Convert the decrypted message bytes to a String and return it
    String::from_utf8(decrypted_msg).map_err(|e| e.to_string())
}

fn append_metadata_to_key(
    file_path: &str,
    file_name: &Option<String>,
    size_in_bytes: u64,
    generation_date: &Option<String>,
) -> Result<(), String> {
    // Extract generation_date
    let date_str = match generation_date {
        Some(date) => date.clone(),
        None => chrono::Utc::now().to_rfc3339(),
    };
    let file_name = match file_name {
        Some(name) => name,
        None => Path::new(file_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown"),
    };

    // Metadata structure
    let metadata = serde_json::json!({
        "filename": file_name,
        "sizeInBytes": size_in_bytes,
        "generationDate": date_str,
        "updateDate": chrono::Utc::now().to_rfc3339(),
    });

    // Convert metadata to a String
    let metadata_str = metadata.to_string();

    // Unique separator
    let separator = "\n---METADATA---\n";

    // println!("\nmetadata: {}", metadata_str);

    // Open the file in append mode to update it
    let mut file = OpenOptions::new()
        .append(true) // Use append instead of truncate
        .open(file_path)
        .map_err(|e| e.to_string())?;

    file.write_all(separator.as_bytes())
        .map_err(|e| e.to_string())?;

    file.write_all(metadata_str.as_bytes())
        .map_err(|e| e.to_string())?;

    file.flush().map_err(|e| e.to_string())?;

    Ok(())
}

fn append_metadata_to_msg(
    encrypted_msg: &mut Vec<u8>,
    file_path: &str,
    file_name: &Option<String>,
    new_key_content_len: usize,
    generation_date: Option<String>,
) {
    let file_name = match file_name {
        Some(name) => name,
        None => Path::new(file_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown"),
    };

    let prepend_metadata = json!({
        "filename": file_name,
        "sizeInBytes": new_key_content_len as u64,
        "generationDate": generation_date.unwrap_or_else(|| Utc::now().to_rfc3339()),
    });

    let prepend_metadata_bytes = prepend_metadata.to_string().into_bytes();

    encrypted_msg.extend_from_slice(b"-->>");
    encrypted_msg.extend_from_slice(&prepend_metadata_bytes);
}
