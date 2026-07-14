use std::{io::Cursor, sync::{Arc, Mutex, mpsc::Sender}};
use std::thread;
use crate::airplay::sinks::gst_gtk4::{GstSinkData};
use crate::airplay::stream_processors::{
    screen,
    audio,
};
use crate::airplay::audio::AudioStreamBasicDescription;
use crate::timing::bindings::AirTunesClockRef;
use rand::Rng;
use sha2::{Sha512};
use hkdf::Hkdf;
use serde::Serialize;
use gst_app::AppSrc;

// get ready cause this is about to get heavy
#[derive(Debug, Clone)]
pub struct AudioStream {
    pub r#type: u64,
    pub stream_connection_id: String,
    pub unfmt_stream_connection_id: plist::Value,
    pub control_port: Option<u64>,
    pub data_port: Option<u32>,
    pub audio_format: u64,
    pub audio_type: String,
    pub spf: u64,
    pub sink: Option<AppSrc>,
    pub pipeline: Option<gst::Pipeline>,
}
#[derive(Debug, Clone)]
pub struct ScreenStream {
    pub r#type: u64,
    pub uuid: Option<String>,
    pub stream_connection_id: String,
}
#[derive(Debug, Clone)]
pub enum Stream {
    Screen(ScreenStream),
    Audio(AudioStream)
}

#[derive(Serialize, Debug, Clone)]
pub struct StreamResponse {
    pub r#type: u64,
    pub dataPort: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    controlPort: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    streamConnectionID: Option<plist::Value>
}
#[derive(Serialize, Debug, Clone)]
pub struct sresp2 {
    pub streams: Vec<StreamResponse>
}

#[derive(Debug, Clone)]
pub struct EventSessionMessage {
    pub content_type: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct AirplaySession {
    // Clock
    pub airtunes_clock: *mut AirTunesClockRef,
    
    // Stream arrays
    pub screen_streams: Vec<Arc<Mutex<ScreenStream>>>,
    pub audio_streams: Vec<Arc<Mutex<AudioStream>>>,
    
    // sink (oh fck me)
    pub sink: Option<Arc<Mutex<GstSinkData>>>,
    
    // Comms
    pub event_channel: Option<Sender<EventSessionMessage>>
}

pub struct PeerData {
    pub session_uuid: String,
    pub session_id: String,

    pub peer_ip: String,
    pub peer_name: String,
    pub peer_mac_address: String,
    pub peer_device_id: String,
    pub peer_timing_port: u64,
}

impl PeerData {
    pub fn new() -> Self {
        Self {
            session_uuid: String::from(""),
            session_id: String::from(""),
            peer_ip: String::from(""),
            peer_name: String::from(""),
            peer_mac_address: String::from(""),
            peer_device_id: String::from(""),
            peer_timing_port: 0,
        }
    }
}

impl AirplaySession {
    pub fn new() -> Self {
        Self {
            // Clock
            airtunes_clock: std::ptr::null_mut(),
            
            // Stream data
            screen_streams: Vec::new(),
            audio_streams: Vec::new(),
            
            // sink
            sink: None,
            
            // Comms
            event_channel: None
        }
    }
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
pub fn process_streams(peer_ip: String, streams: Vec<Stream>, response_body: &mut Vec<u8>, shared_secret: Vec<u8>, as_ref: Arc<AppSrc>) {
    let mut end_stream_data: Vec<StreamResponse> = vec![];
    let mut rng = rand::thread_rng();
    // Check if ScreenStream or AudioStream
    for stream in streams {
        match stream {
            Stream::Screen(sst) => {
                let port = rng.gen_range(1000..=9999);
                let port_string = port.to_string();
                let sc_arc = Arc::new(Mutex::new(sst.clone()));
                let sc_locked = sc_arc.lock().unwrap();
                let ss = shared_secret.clone();
                let sci = sc_locked.stream_connection_id.clone();
                let ac = Arc::clone(&as_ref);
                thread::Builder::new()
                .name("screen-recv-server".to_string()) // Set the desired name for the thread
                .spawn(|| {
                    // This is the code that will run in the new thread
                    screen::init(ss, port_string, sci, ac)
                })
                .expect("Failed to spawn thread");
                end_stream_data.push(StreamResponse {
                    r#type: 110,
                    dataPort: 7777,
                    controlPort: None,
                    streamConnectionID: None,
                });
                println!("[aps] Found a main screen!")
            },
            Stream::Audio(ast) => {
                // Check what kind of audio stream
                match ast.r#type {
                    100 => {
                        println!("[aps] found main audio!");

                        // If the port numbr exists & is above 0 then execute logic for bidirectional audio
                        // else execute the logic for audio out
                        if ast.data_port.is_some() && ast.data_port.unwrap() < 0 {
                            println!("[aps] [main audio] configuring for bidirectional audio");
                        } else {
                            println!("[aps] [main audio] configuring for audio output");
                        }
                    },
                    102 => {
                        let data_port = rng.gen_range(1000..=9999);
                        let rtcp_port = rng.gen_range(1000..=9999);
                        let data_port_string = data_port.to_string();
                        let rtcp_port_string = rtcp_port.to_string();

                        // Start a new thread for our main high receiver
                        let a_st = Arc::new(Mutex::new(ast.clone()));
                        let a_st_clone = Arc::clone(&a_st);
                        let ss = shared_secret.clone();
                        let acp = ast.control_port.clone().unwrap().to_string();
                        let pipc = peer_ip.clone();
                        thread::Builder::new()
                        .name("main-high-recv-server".to_string())
                        .spawn(move || {
                            audio::init(pipc, ss, data_port_string, rtcp_port_string, acp, a_st_clone)
                        })
                        .expect("Failed to spawn thread");

                        // Push the stream
                        end_stream_data.push(StreamResponse {
                            r#type: 102,
                            dataPort: data_port.try_into().unwrap(),
                            controlPort: Some(rtcp_port.try_into().unwrap()),
                            streamConnectionID: Some(ast.unfmt_stream_connection_id)
                        });
                        println!("[aps] found main high audio!");
                    },
                    _ => {}
                }
            }
        }
    };
    let mut rp = sresp2 {
        streams: end_stream_data.clone()
    };
    let mut wri: Vec<u8> = Vec::new();
    plist::to_writer_binary(&mut wri, &rp).expect("Failed to create a binary plist");
    println!("{:?}", plist::Value::from_reader(Cursor::new(&wri)));
    response_body.extend_from_slice(&wri.as_slice());
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