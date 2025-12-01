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
    pub control_port: String,
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
    
    // Stream arrays
    pub screen_streams: Vec<Arc<Mutex<ScreenStream>>>,
    pub audio_streams: Vec<Arc<Mutex<AudioStream>>>,
    
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
            screen_streams: Vec::new(),
            audio_streams: Vec::new(),
            
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
    let mut end_stream_data: Vec<StreamResponse> = vec![];
    let mut aps = airplay_session.lock().unwrap();
    {
        if aps.session_uuid == String::from("") {
            panic!("Tried to process a stream from a (seemingly) empty AirplaySession.");
        }
    };
    // Check if ScreenStream or AudioStream
    for stream in streams {
        match stream {
            Stream::Screen(sst) => {
                // Push ScreenStream to airplay_session.screen_streams
                let sc = Arc::new(Mutex::new(sst.clone()));
                aps.screen_streams.push(Arc::clone(&sc));
                // lock for reading
                let streamdata = sc.lock().unwrap();
                let ss = shared_secret.clone();
                let sci = streamdata.stream_connection_id.clone();
                let aps = airplay_session.clone();
                thread::Builder::new()
                .name("screen-recv-server".to_string()) // Set the desired name for the thread
                .spawn(|| {
                    // This is the code that will run in the new thread
                    screen::init(ss, 7777.to_string(), sci, aps)
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
                    102 => {
                        let mut rng = rand::thread_rng();
                        let random_number = rng.gen_range(1000..=9999);
                        let random_numberc = random_number.clone();
                        let random_number2 = rng.gen_range(1000..=9999);
                        let random_number2c = random_number2.clone();
                        // Push
                        let a_st = Arc::new(Mutex::new(ast.clone()));
                        let a_st_clone = Arc::clone(&a_st);
                        aps.audio_streams.push(Arc::clone(&a_st));
                        let ss = shared_secret.clone();
                        let aps2 = airplay_session.clone();
                        let acp = ast.control_port.clone();
                        thread::Builder::new()
                        .name("main-high-recv-server".to_string())
                        .spawn(move || {
                            audio::init(aps2, ss, random_numberc.to_string(), random_number2c.to_string(), acp, a_st_clone)
                        })
                        .expect("Failed to spawn thread");
                        // please fix somebody this code is probably disgusting
                        end_stream_data.push(StreamResponse {
                            r#type: 102,
                            dataPort: random_number.try_into().unwrap(),
                            controlPort: Some(random_number2.try_into().unwrap()),
                            streamConnectionID: Some(ast.unfmt_stream_connection_id)
                        });
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