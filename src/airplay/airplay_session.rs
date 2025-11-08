use std::sync::{Arc, Mutex};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use threadpool::ThreadPool;
use plist::Value;
use std::thread;
use crate::airplay::stream_processors::screen;
use crate::networking::http_responders::pair_setup::Session;
use crate::networking::http_server::{decrypt_chunk, encrypt_chunk};
use crate::parsing::http::{
    self,
    HttpRequest,
    HttpResponse,
};
use crate::timing::bindings::AirTunesClockRef;
use sha2::{Sha512};
use hkdf::Hkdf;
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, KeyInit, aead::{Aead, Payload, Tag, generic_array::GenericArray}};
use byteorder::{ByteOrder, LittleEndian};
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

fn handle_client(mut stream: TcpStream, shared_secret: Vec<u8>) {
    println!("CAUGHT WITHIN AIRPLAY_SESSION");
    let mut read_counter: u64 = 0;
    loop {
        let a: Vec<String> = Vec::new();
        let mut buffer = [0; 4096];
        let mut data: Vec<u8> = Vec::new();
        
        // right now we are receiving everything but we must parse it before moving to a handler
        let read: usize;
        match stream.read(&mut buffer) {
            Ok(buf) => read=buf,
            Err(e) => { log::error!("Error was caught while reading buffer. Usually this is non fatal and usually means a reset connection: {}", e); return;}
        }
        data.append(&mut buffer[..read].to_vec());
        if data.len() == 0 {
            continue;
        }
        let mut key1 = [0u8; 32];
        Hkdf::<Sha512>::new(Some(b"Events-Salt"), &shared_secret)
        .expand(b"Events-Read-Encryption-Key", &mut key1)
        .expect("HKDF expand failed");
        
        // data layout: [2-byte header][encrypted body][16-byte tag]
        let decrypted = decrypt_chunk(
            &key1,
            &data[0..2],                       // header as AAD
            &data[2..data.len() - 16],         // encrypted body
            &data[data.len() - 16..],          // auth tag
            &mut read_counter
        );
        
        // Replace original buffer if needed
        data = decrypted;
        let request_data: HttpRequest = http::generate_httprequest(&mut data);
        println!("[APS] REQUEST DATA IS AS FOLLOWS ----------- {:?}", data);
        println!("[APS] (PROC) REQUEST DATA IS AS FOLLOWS ----------- {:?}", request_data);
        /*match request_data.path.as_str() {
        //"/info" => {handle_info(request_data);},
        _ => {println!("{}", request_data.path);}
        }*/
        // flush response to client
        match stream.write_all(&[5u8; 36]) {
            Ok(_) => {},
            Err(e) => {log::error!("Error was found while writing header to stream. {}", e); break},
        }
        
        match stream.flush() {
            Ok(_) => {},
            Err(e) => {log::error!("Error was found while flushing to stream. {}", e); break},
        }
    }
}

pub fn init(pair_session: Session) {
    let listener = TcpListener::bind("0.0.0.0:1337").expect("Failed to initialize Airplay Event Session");
    let pool = ThreadPool::new(4);
    log::info!("Initialized HTTP/RSTP server");
    // accept connections and process them serially
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let ss = pair_session.shared_secret.clone();
        
        pool.execute(|| {
            handle_client(stream, ss);
        });
    }
    log::debug!("ENDED!!!!!!!!!!");
}

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
                thread::Builder::new()
                .name("screen-recv-server".to_string()) // Set the desired name for the thread
                .spawn(|| {
                    // This is the code that will run in the new thread
                    screen::init(ss, 7777.to_string(), stream)
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