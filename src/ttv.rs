extern crate hyper;
extern crate hyper_native_tls;
extern crate rustc_serialize;

use std::io::*;
use std::result::Result;
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

    pub fn get_channel_bttv_emotes(&mut self, channel: &str) -> Result<(), String> {

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
