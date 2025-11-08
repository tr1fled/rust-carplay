// moving to this file, because if i dont http_server will turn to 999999999 lines of pure spaghetti
use crate::{airplay::{self, airplay_session::{AirplaySession, Stream}}, networking::http_responders::pair_setup::Session, parsing::http::{
    HttpRequest,
    HttpResponse
}, timing::bindings::{AirTunesClock_Create, AirTunesClockPrivate, AirTunesClockRef}};
use std::sync::{Arc, Mutex};
use plist::Value;
use log;
use serde::{Serialize, Deserialize};
use std::thread;
use std::io::Cursor;

#[derive(Deserialize, Debug)]
struct setup_data {
    features: Vec<String>,
    sourceVersion: String,
    keepAliveLowPower: bool,
    deviceID: String,
    osBuildVersion: String,
    macAddress: String,
    sessionUUID: String,
    name: String,
    timingPort: i32,
}
#[derive(Serialize, Debug)]
struct req_setup_data {
    eventPort: i32,
    enabledFeatures: Vec<String>,
    keepAlivePort: i32,
    timingPort: i32,
}

pub fn handle_setup(request_data: &mut HttpRequest, pair_session: &mut Session, airplay_session: &Arc<Mutex<AirplaySession>>) -> HttpResponse {
    // Start parsing & define startup variables
    let req_parsed = Value::from_reader(Cursor::new(request_data.body.clone())).expect("Failed to parse PLIST");
    println!("{:#?}", req_parsed);
    let session_id: String;
    let mut response_body: Vec<u8> = Vec::new();
    if let Some((key, value)) = request_data.path[7..].split_once("/") {
        session_id = String::from(value);
    } else {
        panic!("Delimiter was not found while parsing SETUP request");
    };
    let mut airtunes_clock: AirTunesClockRef = AirTunesClockRef(std::ptr::null_mut());
    if !req_parsed.as_dictionary().and_then(|dict| dict.get("streams")).is_some() {
        let mut aps = airplay_session.lock().unwrap();
        aps.session_id = session_id;
        aps.session_uuid = String::from(req_parsed.as_dictionary().and_then(|dict| dict.get("sessionUUID")).expect("Failed to fetch sessionUUID").as_string().expect("Failed to convert sessionUUID to String"));
        aps.peer_device_id = String::from(req_parsed.as_dictionary().and_then(|dict| dict.get("deviceID")).expect("Failed to fetch deviceID").as_string().expect("Failed to convert deviceID to String"));
        aps.peer_mac_address = String::from(req_parsed.as_dictionary().and_then(|dict| dict.get("macAddress")).expect("Failed to fetch macAddress").as_string().expect("Failed to convert macAddress to String"));
        aps.peer_timing_port = req_parsed.as_dictionary().and_then(|dict| dict.get("timingPort")).expect("Failed to fetch timingPort").as_unsigned_integer().expect("Failed to convert timingPort to u64");
        println!("creating atc");
        unsafe {
            let err = AirTunesClock_Create(&mut airtunes_clock);
            if err != 0 {
                panic!("Error while creating AirTunes Clock: {}", err);
            }
        }
        plist::to_writer_binary(&mut response_body, &req_setup_data {
            eventPort: 1337,
            enabledFeatures: vec![
            //String::from("uiContext"),
            //String::from("cornerMasks"),
            //String::from("focusTransfer"),
            ],
            keepAlivePort: 1338,
            timingPort: 7000,
        }).expect("Failed to create a binary plist");
        println!("{}", String::from_utf8_lossy(&response_body.as_slice()));
        let aps = Arc::clone(&airplay_session);
        let ps = pair_session.clone();
        let clock_wrapper = airtunes_clock;
        thread::Builder::new()
        .name("airplay_timing_server".to_string()) // Set the desired name for the thread
        .spawn(move || {
            // This is the code that will run in the new thread
            airplay::timing_server::init(ps, aps, clock_wrapper);
        });
        let ps = pair_session.clone();
        thread::Builder::new()
        .name("airplay_session_init".to_string()) // Set the desired name for the thread
        .spawn(|| {
            // This is the code that will run in the new thread
            airplay::airplay_session::init(ps);
        });
    } else {
        println!("Now we have streams to process, sending them to the stream processor...");
        let streams = req_parsed.as_dictionary().and_then(|dict| dict.get("streams")).expect("Failed to fetch streams").as_array().expect("Failed to fetch streams as array");
        let mut streamlist: Vec<Stream> = vec![];
        println!("Processing streams. Streams found will be below!");
        for stream in streams {
            let sci_unsigned: u64 = {
                // Fetch the streamConnectionID from the plist dictionary
                let raw_value = stream.as_dictionary()
                .and_then(|dict| dict.get("streamConnectionID"))
                .expect("Failed to fetch stream connection id");
                
                // Convert it to i64 if possible, otherwise fail
                let signed_id = raw_value
                .as_signed_integer()
                .expect("Failed to convert streamConnectionID to signed integer");
                
                // Reinterpret as u64
                signed_id as u64
            };
            let sci = format!("{}", sci_unsigned);
            let mut end_stream: Stream;
            if stream.as_dictionary().and_then(|dict| dict.get("type")).expect("Failed to fetch stream type").as_unsigned_integer().expect("Failed to fetch stream type as int") == 110 {
                end_stream = Stream {
                    r#type: stream.as_dictionary().and_then(|dict| dict.get("type")).expect("Failed to fetch stream type").as_unsigned_integer().expect("Failed to fetch stream type as int"),
                    audio_type: None,
                    audio_format: None,
                    uuid: Some(String::from(stream.as_dictionary().and_then(|dict| dict.get("uuid")).expect("Failed to fetch stream UUID").as_string().expect("Failed to fetch stream UUID as str"))),
                    stream_connection_id: sci,
                };
            } else {
                end_stream = Stream {
                    r#type: stream.as_dictionary().and_then(|dict| dict.get("type")).expect("Failed to fetch stream type").as_unsigned_integer().expect("Failed to fetch stream type as int"),
                    audio_type: Some(String::from(stream.as_dictionary().and_then(|dict| dict.get("audioType")).expect("Failed to fetch stream's audioType").as_string().expect("Failed to fetch stream's audioType as str"))),
                    audio_format: Some(stream.as_dictionary().and_then(|dict| dict.get("audioFormat")).expect("Failed to fetch stream's audio format'").as_unsigned_integer().expect("Failed to fetch stream's audio format as int'")),
                    stream_connection_id: sci,
                    uuid: None,
                };
            }
            match stream.as_dictionary().and_then(|dict| dict.get("type")).expect("Failed to fetch stream type").as_unsigned_integer().expect("Failed to fetch stream type as int") {
                0 => panic!("Invalid stream type 0"),
                100 => println!("Main audio"),
                101 => println!("Alt audio"),
                102 => println!("Main high audio"),
                106 => println!("Aux out audio"),
                // Main Screen // TCP // 110
                110 => {
                    println!("Found a main screen!")
                },
                111 => println!("Alt screen"),
                _ => panic!("Stream type not recognized"),
            }
            streamlist.push(end_stream);
        }
        println!("Those were the streams.");
        airplay::airplay_session::process_streams(streamlist, &airplay_session, &mut response_body, pair_session.shared_secret.clone());
    }
    println!("afterthread!!!!!!!!!!!!!!!!!!!");
    //airplay::airplay_session::init(sti, airplay_session);
    return HttpResponse {
        status_code: String::from("200 OK"),
        protocol: String::from("RSTP/1.0"),
        headers: vec![],
        body: response_body,
        content_type: String::from("application/x-apple-binary-plist"),
        end: false,
    };
}