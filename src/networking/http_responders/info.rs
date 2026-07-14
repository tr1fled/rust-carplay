use plist::{self, Value, Dictionary, Data};
use std::io::Cursor;
use crate::{networking::http_responders::pair_setup::Session, parsing::http::{HttpRequest, HttpResponse}};
use serde::{Deserialize, Serialize};
use crate::airplay::audio::{AudioFmtData, AudioLatData, populate_audio_formats, populate_audio_latencies};
use crate::persister::{self, CFG_ED25519_PK, CFG_ED25519_SK, CFG_PAIRING_IDENTITY};
use log;

#[derive(Serialize)]
enum MultiTyper {
    Integer(i32),
    Text(String),
    Boolean(bool),
}

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
    bluetoothIDs: Vec<String>,
    deviceID: String,
    displays: Vec<ScreenData>,
    hidDevices: Vec<TouchscreenData>,
    sourceVersion: String,
    features: u64,
    firmwareRevision: String,
    hardwareRevision: String,
    //hidLanguages: Vec<MultiTyper>,
    manufacturer: String,
    modes: Modes,
    name: String,
    nightMode: bool,
    //oemIcon: plist::Data,
    //oemIcons: Vec<u8>,
    //oemIconLabel: String, 
    //oemIconVisible: bool, // uncomment above if set to true, but since it is false...
    OSInfo: String,
    protocolVersion: String,
    rightHandDrive: bool,
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
    r#type: u64,
    maxFPS: u64,
    widthPixels: u64,
    primaryInputDevice: u8,
    heightPhysical: u64,
    heightPixels: u64,
    uuid: String,
    widthPhysical: u64,
    features: u64,
}
#[derive(Serialize)]
struct ModeResource {
    resourceID: u64,
    transferType: u64,
    transferPriority: u64,
    takeConstraint: u64,
    borrowConstraint: u64,
    unborrowConstraint: u64
}
#[derive(Serialize)]
struct ModeAppState {
    appStateID: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    speechMode: Option<i64>,
}
#[derive(Serialize)]
struct Modes {
    resources: Vec<ModeResource>,
    appStates: Vec<ModeAppState>
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
            ],
            statusFlags: 1 << 2,
            bluetoothIDs: vec![
                String::from("c0:d0:12:87:6a:49")
            ],
            deviceID: String::from("DC:A6:32:C3:F3:DE"),
            displays: vec![ScreenData {
                features: 0,
                r#type: 110,
                maxFPS: 60,
                heightPhysical: 0,
                widthPhysical: 0,
                heightPixels: 720,
                widthPixels: 1280,
                uuid: String::from("a5f7a68d-7b0f-4305-984b-974f677a150h"),
                primaryInputDevice: 0x01,
            }],
            hidDevices: vec![TouchscreenData {
                hidProductID: 1,
                hidCountryCode: 0,
                uuid: String::from("e5f7a68d-7b0f-4305-984b-974f677a150b"),
                hidDescriptor: Value::Data(hid_touchscreen_single_descriptor(1280, 720)),
                displayUUID: String::from("a5f7a68d-7b0f-4305-984b-974f677a150h"),
                hidVendorID: 2,
                name: String::from("Touch Screen"),
            }],
            sourceVersion: String::from("490.0"),
            features: 418060152546,
            firmwareRevision: String::from("rust-carplay0.1"),
            hardwareRevision: String::from("rust-carplay0.1"),
            //hidLanguages: vec![],
            manufacturer: String::from("Luixiuno"),
            modes: Modes {
                resources: vec![
                    ModeResource {
                        resourceID: 1,
                        transferType: 1,
                        transferPriority: 100,
                        takeConstraint: 100,
                        borrowConstraint: 100,
                        unborrowConstraint: 100
                    },
                    ModeResource {
                        resourceID: 2,
                        transferType: 1,
                        transferPriority: 100,
                        takeConstraint: 100,
                        borrowConstraint: 100,
                        unborrowConstraint: 100
                    }
                ],
                appStates: vec![
                    ModeAppState {
                        appStateID: 2,
                        state: Some(false),
                        speechMode: None
                    },
                    ModeAppState {
                        appStateID: 1,
                        state: None,
                        speechMode: Some(-1)
                    },
                    ModeAppState {
                        appStateID: 3,
                        state: Some(false),
                        speechMode: None
                    },
                ]
            },
            name: String::from("CarPlay"),
            nightMode: false,
            //oemIcon: plist::Data::new(fs::read("tux.png").unwrap()),
            //oemIcons: Vec<u8>,
            //oemIconLabel: String::from("rust-carplay"), 
            //oemIconVisible: true, // uncomment above if set to true, but since it is false...
            OSInfo: String::from("Linux 6.12.34"),
            protocolVersion: String::from("1.0"),
            rightHandDrive: false,
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

pub fn hid_touchscreen_single_descriptor(width: u16, height: u16) -> Vec<u8> {
    let mut desc: Vec<u8> = vec![
    0x05, 0x0D,         // Usage Page (Digitizer)
    0x09, 0x04,         // Usage (Touch Screen)
    0xA1, 0x01,         // Collection (Application)
    
    0x05, 0x0D,         // Usage Page (Digitizer)
    0x09, 0x22,         // Usage (Finger)
    0xA1, 0x02,         // Collection (Logical)
    
    // Touch
    0x05, 0x0D,         // Usage Page (Digitizer)
    0x09, 0x33,         // Usage (Touch)
    0x15, 0x00,         // Logical Minimum (0)
    0x25, 0x01,         // Logical Maximum (1)
    0x75, 0x01,         // Report Size (1)
    0x95, 0x01,         // Report Count (1)
    0x81, 0x02,         // Input (Data, Variable, Absolute)
    
    // Padding (7 bits constant)
    0x75, 0x07,
    0x95, 0x01,
    0x81, 0x01,
    
    // X position
    0x05, 0x01,         // Usage Page (Generic Desktop)
    0x09, 0x30,         // Usage (X)
    0x15, 0x00,         // Logical Minimum (0)
    0x26, 0xFF, 0x7F,   // Logical Maximum (width placeholder)
    0x75, 0x10,         // Report Size (16)
    0x95, 0x01,         // Report Count (1)
    0x81, 0x02,         // Input (Data, Variable, Absolute)
    
    // Y position
    0x09, 0x31,         // Usage (Y)
    0x15, 0x00,         // Logical Minimum (0)
    0x26, 0xFF, 0x7F,   // Logical Maximum (height placeholder)
    0x75, 0x10,         // Report Size (16)
    0x95, 0x01,         // Report Count (1)
    0x81, 0x02,         // Input (Data, Variable, Absolute)
    
    0xC0,               // End Collection (Logical)
    0xC0,               // End Collection (Application)
    ];
    
    // Patch width and height into the correct positions
    // (same offsets as in the C code)
    desc[0x27] = (width & 0xFF) as u8;
    desc[0x28] = (width >> 8) as u8;
    desc[0x34] = (height & 0xFF) as u8;
    desc[0x35] = (height >> 8) as u8;
    
    desc
}

pub fn handle_info(request_data: HttpRequest, pair_session: &mut Session) -> HttpResponse {
    log::info!("↙️  HTTP Server / Processing /info request");
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
    println!("{:#?}", request_data);
    //println!("{:#?}", req_parsed_2);
    log::info!("↗️  HTTP Server / Sending /info response");
    HttpResponse { 
        status_code: String::from("200 OK"), 
        protocol: String::from("RTSP/1.0"),
        headers: Vec::new(), 
        content_type: String::from("application/x-apple-binary-plist"), 
        body: body, 
        end: false 
    }
}