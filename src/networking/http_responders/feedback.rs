use plist::{self, Value, Dictionary, Data};
use std::io::Cursor;
use std::sync::{
    Arc,
    Mutex
};
use crate::{networking::http_responders::pair_setup::Session, parsing::http::{HttpRequest, HttpResponse}};
use crate::airplay::airplay_session::AirplaySession;
use serde::{Deserialize, Serialize};
use crate::airplay::audio::{AudioFmtData, AudioLatData, populate_audio_formats, populate_audio_latencies};
use crate::HID::touchscreen::hid_touchscreen_single_descriptor;
use crate::persister::{self, CFG_ED25519_PK, CFG_ED25519_SK, CFG_PAIRING_IDENTITY};
use log;

pub fn handle_feedback(request_data: HttpRequest, airplay_session: Arc<Mutex<AirplaySession>>) -> HttpResponse {
    log::info!("↙️  HTTP Server / Processing /feedback request");
    //let req_parsed_2: info_scheme = plist::from_reader(Cursor::new(request_data.body.clone())).expect("Failed to parse PLIST");
    let mut body: Vec<u8> = Vec::new();
    let mut pairing_identity: Vec<u8>;
    match persister::read_key(CFG_PAIRING_IDENTITY) {
        Ok(pid) => pairing_identity = pid,
        Err(e) => panic!("Fatal! Error while trying to fetch the Pairing Identity, \"{}\". While the program can go on, it is a good practice to fix this.", e),
    }
    //let value = plist::to_writer_binary(&mut body, &info_scheme_rsp::new(String::from_utf8(pairing_identity).expect("Failed to convert Vec to String")));
    for byte in &body {
        print!("0x{:02X}, ", byte);
    }
    println!();
    //println!("{:#?}", req_parsed_2);
    log::info!("↗️  HTTP Server / Sending /feedback response");
    HttpResponse { 
        status_code: String::from("200 OK"), 
        protocol: String::from("RTSP/1.0"),
        headers: Vec::new(), 
        content_type: String::from(""), 
        body: body, 
        end: false 
    }
}