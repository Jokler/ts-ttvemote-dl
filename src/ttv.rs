extern crate hyper;
extern crate hyper_native_tls;
extern crate rustc_serialize;

use std::io::*;
use std::result::Result;
use std::fs::File;
use std::fs::OpenOptions;
use self::rustc_serialize::json::Json;
use std::collections::BTreeMap;

use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;

pub struct TTVEmoteData {
    pub template: String,
    pub data: Vec<(String, String)>,
    sub_emotes: Option<Json>,
}

pub struct BTTVEmoteData {
    pub template: String,
    pub data: Vec<(String, String)>,
}

pub struct Config {
    pub global_ttv: bool,
    pub global_bttv: bool,
    pub ttv_channels: Vec<String>,
    pub bttv_channels: Vec<String>,
}

impl TTVEmoteData {
    pub fn new() -> TTVEmoteData {
        TTVEmoteData {
            template: String::new(),
            data: Vec::new(),
            sub_emotes: None,
        }
    }

    pub fn get_global_emotes(&mut self) -> Result<(), String> {

        let obj = match download_json("https://twitchemotes.com/api_cache/v2/global.json") {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        let templates = obj.get("template").unwrap().as_object().unwrap();
        self.template = templates
            .get("small")
            .unwrap()
            .as_string()
            .unwrap()
            .to_owned();

        let emotes = obj.get("emotes").unwrap();

        for (name, value) in emotes.as_object().unwrap().iter() {
            let id = value
                .as_object()
                .unwrap()
                .get("image_id")
                .unwrap()
                .as_u64()
                .unwrap();
            self.data.push((name.to_owned(), id.to_string()));
        }

        Ok(())
    }

    pub fn update_sub_emote_data(&mut self) -> Result<(), String> {

        println!("Downloading data");
        let obj = match download_json("https://twitchemotes.com/api_cache/v2/subscriber.json") {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        let templates = obj.get("template").unwrap().as_object().unwrap();

        self.template = templates
            .get("small")
            .unwrap()
            .as_string()
            .unwrap()
            .to_owned();
        self.sub_emotes = obj.get("channels").cloned();

        Ok(())
    }

    pub fn get_subscriber_emotes(&mut self, channel: &str) -> Result<(), String> {

        if self.sub_emotes == None {
            if let Err(e) = self.update_sub_emote_data() {
                return Err(e);
            }
        }

        let mut count = 0;

        for (channel_name, value) in
            self.sub_emotes
                .as_ref()
                .unwrap()
                .as_object()
                .unwrap()
                .iter() {

            if channel.to_lowercase() != channel_name.to_lowercase() { continue; }

            let emotes = value
                .as_object()
                .unwrap()
                .get("emotes")
                .unwrap()
                .as_array()
                .unwrap();

            for ref emote in emotes.iter() {
                let object = emote.as_object().unwrap();

                let name = object.get("code").unwrap().as_string().unwrap();
                let id = object.get("image_id").unwrap().as_u64().unwrap();

                self.data.push((name.to_owned(), id.to_string()));
                count += 1;
            }
        }
        match count {
            0 => Err("No emotes found".to_owned()),
            _ => Ok(()),
        }
    }
}


impl BTTVEmoteData {
    pub fn new() -> BTTVEmoteData {
        BTTVEmoteData {
            template: String::new(),
            data: Vec::new(),
        }
    }

    pub fn get_global_bttv_emotes(&mut self) -> Result<(), String> {

        let obj = match download_json("https://api.betterttv.net/2/emotes") {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        self.template = obj.get("urlTemplate")
            .unwrap()
            .as_string()
            .unwrap()
            .to_owned()
            .replace("{{image}}", "1x");
        self.template.insert_str(0, "https:");

        let emotes = obj.get("emotes").unwrap().as_array().unwrap();

        for ref emote in emotes.iter() {

            let object = emote.as_object().unwrap();

            let name = object.get("code").unwrap().as_string().unwrap();
            let id = object.get("id").unwrap().as_string().unwrap();
            let img_type = object.get("imageType").unwrap().as_string().unwrap();

            match img_type {
                "png" => self.data.push((name.to_owned(), id.to_string())),
                "gif" => {},
                _     => println!("Unexpected image type: {}", img_type),
            };
        }

        Ok(())
    }

    pub fn get_channel_bttv_emote(&mut self, channel: &str) -> Result<(), String> {

        let obj = match download_json(&format!("https://api.betterttv.net/2/channels/{}",
                                               channel)) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        self.template = obj.get("urlTemplate")
            .unwrap()
            .as_string()
            .unwrap()
            .to_owned()
            .replace("{{image}}", "1x");
        self.template.insert_str(0, "https:");

        let emotes = obj.get("emotes").unwrap().as_array().unwrap();

        for ref emote in emotes.iter() {
            let object = emote.as_object().unwrap();

            let name = object.get("code").unwrap().as_string().unwrap();
            let id = object.get("id").unwrap().as_string().unwrap();
            let img_type = object.get("imageType").unwrap().as_string().unwrap();

            match img_type {
                "png" => self.data.push((name.to_owned(), id.to_string())),
                "gif" => {},
                _     => println!("Unexpected image type: {}", img_type),
            };
        }

        Ok(())
    }
}

impl Config {
    pub fn new() -> Config {
        Config {
            global_ttv: false,
            global_bttv: false,
            ttv_channels: Vec::new(),
            bttv_channels: Vec::new(),
        }
    }

    pub fn read_from_file(&mut self, path: &str) {

        let cfgfile = match OpenOptions::new().read(true).open(path) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", e);
                return;
            }
        };
        let buf_reader = BufReader::new(cfgfile);

        for v in buf_reader.lines() {

            let line = v.unwrap();
            let s: Vec<&str> = line.split(":").collect();
            let data = *s.last().unwrap();

            match *s.first().unwrap() {
                "TTV-Global"   => self.global_ttv = data == "true",
                "TTV-Channel"  => self.ttv_channels.push(String::from(data)),
                "BTTV-Global"  => self.global_bttv = data == "true",
                "BTTV-Channel" => self.bttv_channels.push(String::from(data)),
                ""             => {},
                _              => println!("Invalid Line!({})", *s.first().unwrap()),
            }
        }
    }

    pub fn write_to_file(&mut self, path: &str) {

        let mut cfgfile = match OpenOptions::new().open(path) {
            Ok(v) => v,
            Err(_) => {
                match File::create(path) {
                    Ok(v) => v,
                    Err(e) => {
                        println!("{}", e);
                        return;
                    }
                }
            }
        };

        if let Err(e) = writeln!(cfgfile, "TTV-Global:{}", self.global_ttv.to_string()) {
            println!("{}", e);
        }

        for channel in self.ttv_channels.clone() {
            if let Err(e) = writeln!(cfgfile, "TTV-Channel:{}", channel) {
                println!("{}", e);
            }
        }

        if let Err(e) = writeln!(cfgfile, "BTTV-Global:{}", self.global_ttv.to_string()) {
            println!("{}", e);
        }

        for channel in self.bttv_channels.clone() {
            if let Err(e) = writeln!(cfgfile, "BTTV-Channel:{}", channel) {
                println!("{}", e);
            }
        }
    }
}

fn download_json(url: &str) -> Result<BTreeMap<String, Json>, String> {

    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let client = Client::with_connector(connector);

    let mut res = match client.get(url).send() {
        Ok(v) => v,
        Err(e) => return Err(format!("{}", e)),
    };

    let mut s = String::new();
    if let Err(e) = res.read_to_string(&mut s) {
        return Err(format!("{}", e));
    }

    let data = Json::from_str(&s).unwrap();
    let obj = data.as_object().unwrap();

    match obj.get("status") {
        Some(v) => {
            match v.as_u64().unwrap() {
                200 => return Ok(obj.clone()),
                _ => {
                    Err(obj.get("message")
                            .unwrap()
                            .as_string()
                            .unwrap()
                            .to_owned())
                }
            }
        }
        None => return Ok(obj.clone()),
    }
}
