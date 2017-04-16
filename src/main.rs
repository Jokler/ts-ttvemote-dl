extern crate hyper;
extern crate hyper_native_tls;
extern crate pbr;

use std::io;
use std::io::*;
use std::fs::File;
use std::fs::metadata;
use std::fs::OpenOptions;

use pbr::ProgressBar;
use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;

mod ttv;
use ttv::{TTVEmoteData, BTTVEmoteData, Config};

fn main() {

    let mut input;
    let mut download_ttv = false;
    let mut download_bttv = false;
    let mut emote_data = TTVEmoteData::new();
    let mut bttv_emote_data = BTTVEmoteData::new();

    let mut config = match std::env::args().nth(1) {
        Some(v) => Config::new().create_from_file(&v),
        None    => {
            if read("Read config?(y/N) ").starts_with("y") {
                Config::new().create_from_file(&read("Path: "))
            } else {
                Config::new()
            }
        },
    };

    let edit_config = !read("Edit config?(Y/n) ").starts_with("n");
    if edit_config {

        input = read("Download global emotes?(Y/n) ");
        config.global_ttv = !input.starts_with("n");

        println!("Add subscriber emotes?(Empty to quit)");
        loop {
            input = read("Channel: ");
            if input == "" { break; }

            config.ttv_channels.push(input);
        }

        input = read("Download global bttv emotes?(Y/n) ");
        config.global_bttv = !input.starts_with("n");

        println!("Add channel bttv emotes?(Empty to quit)");
        loop {
            input = read("Channel: ");
            if input == "" { break; }

            config.bttv_channels.push(input);
        }
    }

    if edit_config && !read("Save config?(Y/n) ").starts_with("n") {
        config.write_to_file(&read("Path: "));
    }

    match std::fs::create_dir_all("./twitchemotes/emoticons/") {
        Ok(_) => println!("Created twitchemotes directory"),
        Err(e) => {
            println!("Failed to create directory: {}", e);
            return;
        }
    }

    if config.global_ttv {
        match emote_data.get_global_emotes() {
            Ok(_) => download_ttv = true,
            Err(e) => println!("Error({})", e),
        }
    }

    if config.global_bttv {
        match bttv_emote_data.get_global_bttv_emotes() {
            Ok(_) => download_bttv = true,
            Err(e) => println!("Error({})", e),
        }
    }

    if !config.ttv_channels.is_empty() {
        match emote_data.update_sub_emote_data() {
            Ok(_) => {
                for channel in &config.ttv_channels {
                    match emote_data.get_subscriber_emotes(&channel) {
                        Ok(_) => download_ttv = true,
                        Err(e) => println!("Error({})", e),
                    }
                }
            },
            Err(e) => println!("Error({})", e),
        }
    }

    if !config.bttv_channels.is_empty() {
        for channel in &config.bttv_channels {
            match bttv_emote_data.get_channel_bttv_emotes(&channel) {
                Ok(_) => download_bttv = true,
                Err(e) => println!("Error({})", e),
            }
        }
    }

    if download_ttv {
        println!("Downloading ttv emotes");
        save_images(&emote_data.template, &mut emote_data.data);
    }
    if download_bttv {
        println!("Downloading bttv emotes");
        save_images(&bttv_emote_data.template, &mut bttv_emote_data.data);
    }
}

// Saves the images, adds them to emoticons.txt and clears the emotes Vec
fn save_images(template: &String, emotes: &mut Vec<(String, String)>) {

    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let client = Client::with_connector(connector);

    let mut txtfile = match OpenOptions::new()
              .append(true)
              .open("./twitchemotes/emoticons/emoticons.txt") {
        Ok(v) => v,
        Err(_) => {
            match File::create("./twitchemotes/emoticons/emoticons.txt") {
                Ok(v) => v,
                Err(e) => {
                    println!("{}", e);
                    return;
                }
            }
        }
    };

    println!("Downloading emotes - existing files will be skipped!");

    let mut pb = ProgressBar::new(emotes.len() as u64);
    pb.format("╢▌▌░╟");

    for &(ref name, ref id) in emotes.iter() {

        // Skip existing files
        if let Ok(_) = metadata(format!("{}{}.{}", "./twitchemotes/emoticons/", id, "png")) {
            continue;
        };

        let url = template
            .replace("{image_id}", &id.to_string())
            .replace("{{id}}", &id.to_string());

        // Download and save the emote
        let mut img = match client.get(&*url).send() {
            Ok(v) => v,
            Err(e) => {
                println!("Error({})", e);
                continue;
            }
        };

        let mut f =
            match File::create(format!("{}{}.{}", "./twitchemotes/emoticons/", id, "png")) {
                Ok(v) => v,
                Err(e) => {
                    println!("{}", e);
                    continue;
                }
            };

        if let Err(e) = io::copy(&mut img, &mut f) {
            println!("Failed to write to file: {}", e);
            continue;
        };

        // Write the emote names into emoticons.txt for TS3
        if let Err(e) = writeln!(txtfile, "{}.png = \"{}\"", &id.to_string(), name) {
            println!("{}", e);
        }

        // Update progress bar
        pb.inc();
    }
    emotes.clear();
    pb.finish_print("Done");
}

// Prints text and returns user input
fn read(text: &str) -> String {
    let mut input = String::new();

    print!("{}", text);

    let _ = stdout().flush();
    stdin()
        .read_line(&mut input)
        .expect("Did not enter a correct string");

    if let Some('\n') = input.chars().next_back() {
        input.pop();
    }
    if let Some('\r') = input.chars().next_back() {
        input.pop();
    }

    input.to_lowercase().trim().to_owned()
}
