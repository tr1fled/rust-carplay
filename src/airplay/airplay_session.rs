use std::{sync::{Arc, Mutex, mpsc::{Sender}}};
use std::thread;
use crate::airplay::sinks::gst_gtk4::{GstSinkData};
use crate::airplay::stream_processors::screen;
use crate::timing::bindings::AirTunesClockRef;
use sha2::{Sha512};
use hkdf::Hkdf;
use serde::Serialize;

// get ready cause this is about to get heavy
#[derive(Serialize, Debug, Clone)]
pub struct Stream {
    pub r#type: u64,
    pub audio_type: Option<String>,
    pub audio_format: Option<u64>,
    pub uuid: Option<String>,
    pub stream_connection_id: String,
}
#[derive(Serialize, Debug, Clone)]
pub struct StreamResponse {
    pub r#type: u64,
    pub dataPort: u64,
}
#[derive(Serialize, Debug, Clone)]
pub struct Streams {
    pub streams: Vec<StreamResponse>
}

#[derive(Debug, Clone)]
pub struct EventSessionMessage {
    pub content_type: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct AirplaySession {
    // General
    pub session_uuid: String,
    pub session_id: String,
    
    // Clock
    pub airtunes_clock: *mut AirTunesClockRef,
    
    // Peer
    pub peer_ip: String,
    pub peer_name: String,
    pub peer_mac_address: String,
    pub peer_device_id: String,
    pub peer_timing_port: u64,
    
    // Stream data
    pub streams: Vec<Stream>,
    
    // sink (oh fck me)
    pub sink: Option<Arc<Mutex<GstSinkData>>>,
    
    // Comms
    pub event_channel: Option<Sender<EventSessionMessage>>
}
impl AirplaySession {
    pub fn new() -> Self {
        Self {
            // General
            session_uuid: String::from(""),
            session_id: String::from(""),
            
            // Clock
            airtunes_clock: std::ptr::null_mut(),
            
            // Peer
            peer_ip: String::from(""),
            peer_name: String::from(""),
            peer_mac_address: String::from(""),
            peer_device_id: String::from(""),
            peer_timing_port: 0,
            
            // Stream data
            streams: Vec::new(),
            
            // sink
            sink: None,
            
            // Comms
            event_channel: None
        }
    }
    /*pub fn new(session_uuid: String, pn: String, pmac: String, pid: String, ptp: u32) -> Self {
    Self {
    // General
    session_uuid,
    
    // Peer
    peer_name: pn,
    peer_mac_address: pmac,
    peer_device_id: pid,
    peer_timing_port: ptp,
    
    // Stream data
    streams: Vec::new(),
    }
    }*/
}
unsafe impl Send for AirplaySession {}
unsafe impl Sync for AirplaySession {}

pub const kAirPlayStreamType_MainAudio: u32 = 	  100; // RTP payload type for low-latency audio input/output. UDP.
pub const kAirPlayStreamType_AltAudio: u32 = 	  101; // RTP payload type for low-latency UI sounds, alerts, etc. output. UDP.
pub const kAirPlayStreamType_MainHighAudio: u32 = 102; // RTP payload type for high-latency audio output. UDP.
pub const kAirPlayStreamType_Screen: u32 = 		  110; // RTP payload type for H.264 screen output. TCP.

/// Process stream data
/// 
/// # Arguments
/// * `streams` - `Vec<Stream>` containing each stream that should be processed, usually one but array says stream(S)
/// * `airplay_session` - Borrowed `AirplaySession` struct wrapped in `Arc<Mutex<>>`
/// * `response_vody` - Borrowed & mutable Vec<u8> which will be used to write the response body
/// * `shared_secret` - The shared secret from the pairing session
pub fn process_streams(streams: Vec<Stream>, airplay_session: &Arc<Mutex<AirplaySession>>, response_body: &mut Vec<u8>, shared_secret: Vec<u8>) {
    let mut end_stream_data = Streams {
        streams: vec![],
    };
    {
        if airplay_session.lock().unwrap().session_uuid == String::from("") {
            panic!("Tried to process a stream from a (seemingly) empty AirplaySession.");
        }
    };
    for stream in streams {
        match stream.r#type {
            0 => panic!("Invalid stream type 0"),
            100 => println!("Main audio"),
            101 => {
                println!("Alt audio")
            },
            102 => println!("Main high audio"),
            106 => println!("Aux out audio"),
            // Main Screen // TCP // 110
            110 => {
                let ss = shared_secret.clone();
                let aps = airplay_session.clone();
                thread::Builder::new()
                .name("screen-recv-server".to_string()) // Set the desired name for the thread
                .spawn(|| {
                    // This is the code that will run in the new thread
                    screen::init(ss, 7777.to_string(), stream, aps)
                })
                .expect("Failed to spawn thread");
                end_stream_data.streams.push(StreamResponse {
                    r#type: 110,
                    dataPort: 7777,
                });
                println!("Found a main screen!")
            },
            111 => println!("Alt screen"),
            _ => panic!("Stream type not recognized"),
        }
    };
    plist::to_writer_binary(response_body, &end_stream_data).expect("Failed to create a binary plist");
}

/// Derive an encryption key using HKDF, defaults to 32-byte long key
/// 
/// # Arguments
/// * `shared_secret` - `Vec<u8>` containing the shared secret acquired from the pairing process
/// * `stream_id` - `String` containing the stream ID, required for key derivation process, otherwise result will be invalid
/// 
/// # Returns
/// * `encryption_key` - `[u8; 32]` containing the derived encryption key
pub fn derive_stream_key(shared_secret: Vec<u8>, stream_id: String) -> [u8; 32] {
    let mut res = [0u8; 32];
    Hkdf::<Sha512>::new(Some(format!("DataStream-Salt{}", stream_id).as_bytes()), &shared_secret)
    .expand(b"DataStream-Output-Encryption-Key", &mut res)
    .expect("HKDF expand failed");
    res
}