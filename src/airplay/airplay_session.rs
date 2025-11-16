use std::{sync::{Arc, Mutex, mpsc::{self, Receiver, Sender, TryRecvError}}, time::Duration};
use std::io::{Read, Write, ErrorKind::WouldBlock};
use std::net::{TcpListener, TcpStream};
use threadpool::ThreadPool;
use plist::Value;
use std::thread;
use crate::airplay::sinks::gst_gtk4::{GstSinkData, init_sink};
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

fn decrypt_data(data: Vec<u8>, shared_secret: &Vec<u8>, read_counter: &mut u64) -> Vec<u8> {
    let mut key1 = [0u8; 32];
    Hkdf::<Sha512>::new(Some(b"Events-Salt"), &shared_secret)
    .expand(b"Events-Read-Encryption-Key", &mut key1)
    .expect("HKDF expand failed");
    
    // data layout: [2-byte header][encrypted body][16-byte tag]
    let mut decrypted = decrypt_chunk(
        &key1,
        &data[0..2],                       // header as AAD
        &data[2..data.len() - 16],         // encrypted body
        &data[data.len() - 16..],          // auth tag
        read_counter
    );
    
    // Replace original buffer if needed
    decrypted
}

fn encrypt_data(data: Vec<u8>, shared_secret: &Vec<u8>, read_counter: &mut u64) -> Vec<u8> {
    let mut key1 = [0u8; 32];
    Hkdf::<Sha512>::new(Some(b"Events-Salt"), &shared_secret)
    .expand(b"Events-Write-Encryption-Key", &mut key1)
    .expect("HKDF expand failed");
    
    let mut resp_buf: Vec<u8> = Vec::new();
    let (aad, chunk, auth_tag) = encrypt_chunk(&key1, &data, read_counter).expect("[FreakBob] calls again");
    resp_buf.extend_from_slice(&aad);
    resp_buf.extend_from_slice(&chunk);
    resp_buf.extend_from_slice(&auth_tag);
    resp_buf
}

fn handle_client(mut stream: TcpStream, shared_secret: Vec<u8>, airplay_session: Arc<Mutex<AirplaySession>>) {
    let mut read_counter: u64 = 0;
    stream.set_nonblocking(true).unwrap();
    let (sender, receiver) = mpsc::channel();
    {
        let mut aps = airplay_session.lock().unwrap();
        aps.event_channel = Some(sender.clone());
    }
    loop {
        // incoming data?
        match receiver.recv() {
            Ok(esm) => {
                println!("caught in the undertow!!!!!");
                let mut dta = format!("POST /command RTSP/1.0\nAirPlay-Receiver-Device-ID: 242606374384606\nContent-Type: {}\nContent-Length: {}\r\n\r\n", esm.content_type, esm.data.len()).into_bytes();
                dta.extend_from_slice(esm.data.as_slice());
                let mut request = encrypt_data(dta, &shared_secret, &mut read_counter);
                match stream.write_all(&request) {
                    Ok(_) => {},
                    Err(e) => {log::error!("Error was found while writing header to stream. {}", e); break},
                }
                
                match stream.flush() {
                    Ok(_) => {},
                    Err(e) => {log::error!("Error was found while flushing to stream. {}", e); break},
                }
            }
            Err(err) => {
                thread::sleep(Duration::from_millis(200));
            }
            /*Err(TryRecvError::Disconnected) => {
                eprintln!("[Event Server] All threads have dropped sender, dropping.");
                break;
            }*/
        }
        let a: Vec<String> = Vec::new();
        let mut buffer = [0; 4096];
        let mut data: Vec<u8> = Vec::new();
        
        // Check if we can read data from the non blocking TCP stream
        // If there is none, go ahead to the mpsc checker
        let read: usize;
        match stream.read(&mut buffer) {
            Ok(buf) => {
                read=buf;
                data.append(&mut buffer[..read].to_vec());
                if data.len() == 0 {
                } else {
                    //println!("{:?}", String::from_utf8(decrypt_data(data, &shared_secret, &mut read_counter)));
                    println!("decrypted the response?");
                }
            },
            Err(e) => {
                if e.kind() == WouldBlock {
                    
                }else {
                    log::error!("Error was caught while reading buffer. Usually this is non fatal and usually means a reset connection: {}", e);
                    return;
                }
            }
        }
    }
}

pub fn init(pair_session: Session, airplay_session: Arc<Mutex<AirplaySession>>) {
    let listener = TcpListener::bind("0.0.0.0:1337").expect("Failed to initialize Airplay Event Session");
    let pool = ThreadPool::new(4);
    log::info!("Initialized HTTP/RSTP server");
    // accept connections and process them serially
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let ss = pair_session.shared_secret.clone();
        let aps = Arc::clone(&airplay_session);
        
        pool.execute(|| {
            handle_client(stream, ss, aps);
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