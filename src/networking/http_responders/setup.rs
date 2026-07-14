// moving to this file, because if i dont http_server will turn to 999999999 lines of pure spaghetti
use crate::{airplay::{self, airplay_session::{AirplaySession, AudioStream, EventSessionMessage, PeerData, ScreenStream, Stream}}, networking::http_responders::pair_setup::Session, parsing::http::{
    HttpRequest,
    HttpResponse
}, timing::bindings::{AirTunesClock_Create, AirTunesClockPrivate, AirTunesClockRef}};
use std::sync::{Arc, Mutex, RwLock, mpsc::Receiver};
use gst_app::AppSrc;
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

pub fn handle_setup(request_data: &mut HttpRequest, pair_session: &mut Session, peer_data: Arc<RwLock<PeerData>>, rx: Arc<Mutex<Receiver<EventSessionMessage>>>, as_ref: Arc<AppSrc>) -> HttpResponse {
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
        {
            let mut pd = peer_data.write().unwrap();
            pd.session_id = session_id;
            pd.session_uuid = String::from(req_parsed.as_dictionary().and_then(|dict| dict.get("sessionUUID")).expect("Failed to fetch sessionUUID").as_string().expect("Failed to convert sessionUUID to String"));
            pd.peer_device_id = String::from(req_parsed.as_dictionary().and_then(|dict| dict.get("deviceID")).expect("Failed to fetch deviceID").as_string().expect("Failed to convert deviceID to String"));
            pd.peer_mac_address = String::from(req_parsed.as_dictionary().and_then(|dict| dict.get("macAddress")).expect("Failed to fetch macAddress").as_string().expect("Failed to convert macAddress to String"));
            pd.peer_timing_port = req_parsed.as_dictionary().and_then(|dict| dict.get("timingPort")).expect("Failed to fetch timingPort").as_unsigned_integer().expect("Failed to convert timingPort to u64");
        }
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
                //String::from("altScreen"),
            ],
            keepAlivePort: 1338,
            timingPort: 7060,
        }).expect("Failed to create a binary plist");
        println!("{}", String::from_utf8_lossy(&response_body.as_slice()));

        let ps = pair_session.clone();
        let pd = Arc::clone(&peer_data);
        let clock_wrapper = airtunes_clock;
        thread::Builder::new()
        .name("airplay_timing_server".to_string()) // Set the desired name for the thread
        .spawn(move || {
            // This is the code that will run in the new thread
            airplay::timing_server::init(ps, pd, clock_wrapper);
        }).unwrap();

        let ps = pair_session.clone();
        let rx_arc = Arc::clone(&rx);
        thread::Builder::new()
        .name("event_server".to_string())
        .spawn(|| {
            airplay::event_server::init(ps, rx_arc);
        }).unwrap();
    } else {
        println!("Now we have streams to process, sending them to the stream processor...");
        let streams = req_parsed.as_dictionary().and_then(|dict| dict.get("streams")).expect("Failed to fetch streams").as_array().expect("Failed to fetch streams as array");
        let mut streamlist: Vec<Stream> = vec![];
        println!("Processing streams. Streams found will be below!");
        for stream in streams {
            let raw_value = stream.as_dictionary()
            .and_then(|dict| dict.get("streamConnectionID"))
            .expect("Failed to fetch stream connection id");
            let sci_unsigned: u64 = {
                // Convert it to i64 if possible, otherwise fail
                let signed_id = raw_value
                .as_signed_integer()
                .expect("Failed to convert streamConnectionID to signed integer");
                
                // Reinterpret as u64
                signed_id as u64
            };
            let sci_val = raw_value.clone();
            let sci = format!("{}", sci_unsigned);
            let mut end_stream: Stream;
            if stream.as_dictionary().and_then(|dict| dict.get("type")).expect("Failed to fetch stream type").as_unsigned_integer().expect("Failed to fetch stream type as int") == 110 {
                end_stream = Stream::Screen(ScreenStream {
                    r#type: stream.as_dictionary().and_then(|dict| dict.get("type")).expect("Failed to fetch stream type").as_unsigned_integer().expect("Failed to fetch stream type as int"),
                    uuid: Some(String::from(stream.as_dictionary().and_then(|dict| dict.get("uuid")).expect("Failed to fetch stream UUID").as_string().expect("Failed to fetch stream UUID as str"))),
                    stream_connection_id: sci,
                });
            } else {
                end_stream = Stream::Audio(AudioStream {
                    r#type: stream.as_dictionary().and_then(|dict| dict.get("type"))
                    .expect("Failed to fetch stream type")
                    .as_unsigned_integer()
                    .expect("Failed to fetch stream type as int"),
                    audio_type: String::from(stream.as_dictionary().and_then(|dict| dict.get("audioType"))
                    .expect("Failed to fetch stream's audioType")
                    .as_string()
                    .expect("Failed to fetch stream's audioType as str")),
                    audio_format: stream.as_dictionary().and_then(|dict| dict.get("audioFormat"))
                    .expect("Failed to fetch stream's audio format'")
                    .as_unsigned_integer()
                    .expect("Failed to fetch stream's audio format as int'"),
                    stream_connection_id: sci,
                    unfmt_stream_connection_id: sci_val,
                    control_port: Some(stream.as_dictionary().and_then(|dict| dict.get("controlPort"))
                    .expect("Failed to fetch stream's control port'")
                    .as_unsigned_integer().expect("Failed to fetch stream's control port as int'")),
                    data_port: None,
                    spf: stream.as_dictionary().and_then(|dict| dict.get("spf")).expect("Failed to fetch stream's spf'").as_unsigned_integer().expect("Failed to fetch stream's spf as int'").try_into().unwrap(),
                    sink: None,
                    pipeline: None,
                });
            }
            /*match stream.as_dictionary().and_then(|dict| dict.get("type")).expect("Failed to fetch stream type").as_unsigned_integer().expect("Failed to fetch stream type as int") {
                0 => panic!("Invalid stream type 0"),
                100 => {
                    println!("Main audio")
                },
                101 => println!("Alt audio"),
                102 => println!("Main high audio"),
                106 => println!("Aux out audio"),
                // Main Screen // TCP // 110
                110 => {
                    println!("Found a main screen!")
                },
                111 => println!("Alt screen"),
                _ => panic!("Stream type not recognized"),
            }*/
            streamlist.push(end_stream.clone());
            println!("{:?}", end_stream);
        }
        println!("Those were the streams.");
        let peer_ip: String;
        {
            let mut pd = peer_data.read().unwrap();
            peer_ip = pd.peer_ip.clone();
        }
        airplay::airplay_session::process_streams(peer_ip, streamlist, &mut response_body, pair_session.shared_secret.clone(), Arc::clone(&as_ref));
    }
    println!("afterthread!!!!!!!!!!!!!!!!!!!");
    println!("{:#?}", response_body);
    return HttpResponse {
        status_code: String::from("200 OK"),
        protocol: String::from("RSTP/1.0"),
        headers: vec![],
        body: response_body,
        content_type: String::from("application/x-apple-binary-plist"),
        end: false,
    };
}