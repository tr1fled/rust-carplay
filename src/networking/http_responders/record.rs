use plist::{self, Value, Dictionary, Data};
use std::io::Cursor;
use crate::{networking::http_responders::pair_setup::Session, parsing::http::{HttpRequest, HttpResponse}};
use serde::{Deserialize, Serialize};
use crate::airplay::audio::{AudioFmtData, AudioLatData, populate_audio_formats, populate_audio_latencies};
use crate::HID::touchscreen::hid_touchscreen_single_descriptor;
use crate::persister::{self, CFG_ED25519_PK, CFG_ED25519_SK, CFG_PAIRING_IDENTITY};
use log;

#[derive(Deserialize, Debug, Default)]
#[serde(default)] // these are all NOT OPTIONAL! so if we dont find them, they will be made empty
struct info_scheme {
    uiContextURLs: Vec<String>,
    qualifier: Vec<String>,
    altScreenURLs: Vec<String>,
}

#[derive(Serialize)]
struct info_scheme_rsp {
    pi: String,
    audioLatencies: Vec<AudioLatData>,
    keepAliveLowPower: bool,
    model: String,
    keepAliveSendStatsAsBody: bool,
    audioFormats: Vec<AudioFmtData>,
    extendedFeatures: Vec<String>,
    statusFlags: u64,
    //bluetoothIDs: Vec<String>,
    deviceID: String,
    displays: Vec<ScreenData>,
    hidDevices: Vec<TouchscreenData>,
    sourceVersion: String,
    features: u64,
    //firmwareRevision: String,
    //hardwareRevision: String,
    //hidLanguages: Vec<MultiTyper>,
    //manufacturer: String,
    //modes: Vec<MultiTyper>,
    name: String,
    //nightMode: bool,
    //oemIcon: Vec<u8>,
    //oemIcons: Vec<u8>,
    //oemIconLabel: String, 
    //oemIconVisible: bool, // uncomment above if set to true, but since it is false...
    //OSInfo: String,
    //protocolVersion: String,
    //rightHandDrive: bool,
    //vehicleInformation: Vec<MultiTyper>,
    //appearanceDefault: String,
    //buttonInfo: Vec<MultiTyper>,
    //enhancedSiriInfo: Vec<EnhancedSiriEntry>,
}

#[derive(Serialize)]
struct EnhancedSiriEntry {
    enhancedSiriVoice: bool,
    keepAliveLowPower: bool,
}
#[derive(Serialize)]
struct TouchscreenData {
    hidProductID: u64,
    hidCountryCode: u64,
    uuid: String,
    hidDescriptor: Value,
    displayUUID: String,
    hidVendorID: u64,
    name: String,
}
#[derive(Serialize)]
struct ScreenData {
    maxFPS: u64,
    widthPixels: u64,
    primaryInputDevice: u8,
    heightPhysical: u64,
    heightPixels: u64,
    uuid: String,
    widthPhysical: u64,
    features: u64,
}

impl info_scheme_rsp {
    pub fn new(pairing_identity: String) -> Self {
        Self {
            pi: pairing_identity,
            audioLatencies: populate_audio_latencies(),
            keepAliveLowPower: true,
            model: String::from("AirPlayGeneric1,1"),
            keepAliveSendStatsAsBody: true,
            audioFormats: populate_audio_formats(),
            extendedFeatures: vec![
                String::from("vocoderInfo"),
                String::from("theUpdatePushed!"),
            ],
            statusFlags: 1 << 2,
            /*bluetoothIDs: vec![
                String::from("c0:d0:12:87:6a:49")
            ],*/
            deviceID: String::from("DC:A6:32:C3:F3:DE"),
            displays: vec![ScreenData {
                features: 30,
                maxFPS: 60,
                heightPhysical: 0,
                widthPhysical: 0,//
                heightPixels: 1080,
                widthPixels: 1920,
                uuid: String::from("a5f7a68d-7b0f-4305-984b-974f677a150b"),
                primaryInputDevice: 0,
            }],
            hidDevices: vec![TouchscreenData {
                hidProductID: 0,
                hidCountryCode: 0,
                uuid: String::from("e5f7a68d-7b0f-4305-984b-974f677a150b"),
                hidDescriptor: Value::Data(hid_touchscreen_single_descriptor(1920, 1080)),
                displayUUID: String::from("a5f7a68d-7b0f-4305-984b-974f677a150b"),
                hidVendorID: 0,
                name: String::from("Touch Screen"),
            }],
            sourceVersion: String::from("320.17"),
            features: 416679199360,
            //firmwareRevision: String::from("rust-carplay0.1"),
            //hardwareRevision: String::from("rust-carplay0.1"),
            //hidLanguages: vec![],
            //manufacturer: String::from("Luixiuno"),
            //modes: vec![],
            name: String::from("CarPlay"),
            //nightMode: false,
            //oemIcon: Vec<u8>,
            //oemIcons: Vec<u8>,
            //oemIconLabel: String, 
            //oemIconVisible: false, // uncomment above if set to true, but since it is false...
            //OSInfo: String::from("Linux 6.12.34"),
            //protocolVersion: String::from("1.0"),
            //rightHandDrive: false,
            //vehicleInformation: vec![],
            //appearanceDefault: String,
            //buttonInfo: vec![],
            /*enhancedSiriInfo: vec![EnhancedSiriEntry {
                enhancedSiriVoice: false,
                keepAliveLowPower: true,
            }],*/
        }
    }
}

pub fn handle_record(request_data: HttpRequest) -> HttpResponse {
    log::info!("↙️  HTTP Server / Processing RECORD request");
    //let req_parsed_2: info_scheme = plist::from_reader(Cursor::new(request_data.body.clone())).expect("Failed to parse PLIST");
    let mut body: Vec<u8> = Vec::new();
    let mut pairing_identity: Vec<u8>;
    match persister::read_key(CFG_PAIRING_IDENTITY) {
        Ok(pid) => pairing_identity = pid,
        Err(e) => panic!("Fatal! Error while trying to fetch the Pairing Identity, \"{}\". While the program can go on, it is a good practice to fix this.", e),
    }
    let value = plist::to_writer_binary(&mut body, &info_scheme_rsp::new(String::from_utf8(pairing_identity).expect("Failed to convert Vec to String")));
    for byte in &body {
        print!("0x{:02X}, ", byte);
    }
    println!();
    //println!("{:#?}", req_parsed_2);
    log::info!("↗️  HTTP Server / Sending RECORD response");
    HttpResponse { 
        status_code: String::from("200 OK"), 
        protocol: String::from("RTSP/1.0"),
        headers: Vec::new(), 
        content_type: String::from("application/x-apple-binary-plist"), 
        body: body, 
        end: false 
    }
}