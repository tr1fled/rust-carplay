use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::str::Lines;
use chacha20poly1305::aead::AeadMutInPlace;
use log;
use crate::airplay::sinks::gst_gtk4::{GstSinkData, init_sink};
use crate::parsing::http::{
    self,
    HttpRequest,
    HttpResponse,
};
use crate::airplay::airplay_session::AirplaySession;
use crate::networking::{
    http_responders::{
        pair_setup::{handle_pair_setup, Session},
        pair_verify::handle_pair_verify,
        auth_setup::handle_auth_setup,
        info::handle_info,
        record::handle_record,
        setup::handle_setup,
    },
};
use std::{ptr, thread};
use threadpool::ThreadPool;
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, KeyInit, aead::{Aead, Payload, Tag, generic_array::GenericArray}};
use byteorder::{ByteOrder, LittleEndian};
use sha2::{Sha512};
use hkdf::Hkdf;

const SERVER_NAME: &str = "AirTunes/366.0";

pub fn decrypt_chunk(
    read_key: &[u8; 32],
    aad: &[u8],
    data: &[u8],
    auth_tag: &[u8],
    count: &mut u64,
) -> Vec<u8> {
    assert_eq!(auth_tag.len(), 16, "Auth tag must be 16 bytes");
    
    // Initialize AEAD
    let aead = ChaCha20Poly1305::new(Key::from_slice(read_key));
    
    // Construct 12-byte nonce: 4 zero bytes + 8-byte little-endian counter
    let mut nonce_bytes = [0u8; 12];
    LittleEndian::write_u64(&mut nonce_bytes[4..], *count);
    *count += 1;
    let nonce = GenericArray::from_slice(&nonce_bytes);
    
    // Combine ciphertext + auth tag
    let mut ciphertext_with_tag = Vec::with_capacity(data.len() + auth_tag.len());
    ciphertext_with_tag.extend_from_slice(data);
    ciphertext_with_tag.extend_from_slice(auth_tag);
    
    // Perform decryption
    let plaintext = aead
    .decrypt(nonce, Payload { msg: &ciphertext_with_tag, aad })
    .expect("Decryption failed");
    
    plaintext
}

pub fn encrypt_chunk(write_key: &[u8; 32], data: &[u8], count: &mut u64) -> Result<([u8; 2], Vec<u8>, [u8; 16]), chacha20poly1305::Error> {
    let mut aead = ChaCha20Poly1305::new(GenericArray::from_slice(write_key));
    
    let mut nonce = vec![0; 4];
    let mut suffix = vec![0; 8];
    LittleEndian::write_u64(&mut suffix, *count);
    nonce.extend(suffix);
    *count += 1;
    
    let mut aad = [0; 2];
    LittleEndian::write_u16(&mut aad, data.len() as u16);
    
    let mut buffer = Vec::new();
    buffer.extend_from_slice(data);
    let auth_tag = aead.encrypt_in_place_detached(Nonce::from_slice(&nonce), &aad, &mut buffer)?;
    
    Ok((aad, buffer, auth_tag.into()))
}

fn handle_client(mut stream: TcpStream) {
    let mut pair_session: Session = Session::new();
    let mut airplay_session = Arc::new(Mutex::new(AirplaySession::new()));
    let mut sinkdata = Arc::new(Mutex::new(GstSinkData::new()));
    {
        let mut aps = airplay_session.lock().unwrap();
        let sdc = sinkdata.clone();
        aps.sink = Some(sdc);
    }
    let mut csd = sinkdata.clone();
    gst::init().unwrap();
    // right after gst is initialized, we will define the appsrc
    { 
        let caps = gst::Caps::builder("video/x-h264")
        .field("stream-format", "byte-stream") // <-- AnnexB
        .field("alignment", "nal")
        .build();
        let mut data = csd.lock().unwrap();
        data.data_sink = Some(gst_app::AppSrc::builder()
        .caps(&caps)
        .format(gst::Format::Time)
        .build());
    }
    let aps = airplay_session.clone();
    thread::spawn(move || {
        init_sink(aps)
    });
    match stream.peer_addr() {
        Ok(peer_ip) => {
            let mut pip = String::new();
            match peer_ip.to_string().split_once(":") {
                Some((ip, _)) => pip=ip.to_string(),
                None => panic!("Failed to split IP")
            };
            airplay_session.lock().unwrap().peer_ip = pip.clone();
            sinkdata.lock().unwrap().peer_ip = pip.clone();
        },
        Err(error) => panic!("[FIXED LATER (FIXME)] Failed to get the peer's IP address. The server cannot live on unless fixed: {}", error)
    }
    println!("caught");
    loop {
        println!("loop catches");
        let a: Vec<String> = Vec::new();
        let mut buffer = [0; 4096];
        let mut data: Vec<u8> = Vec::new();
        
        // right now we are receiving everything but we must parse it before moving to a handler
        let read: usize;
        match stream.read(&mut buffer) {
            Ok(buf) => read=buf,
            Err(e) => { log::error!("Error was caught while reading buffer. Usually this is non fatal and usually means a reset connection: {}", e); break }
        }
        data.append(&mut buffer[..read].to_vec());
        if data.len() == 0 { // enough for a magic number of RSTP or HTTP
            match stream.write_all(&[0]) {
                Ok(_) => {},
                Err(e) => {log::error!("Error was found while writing header to stream. {}", e); break},
            }
            
            match stream.flush() {
                Ok(_) => {},
                Err(e) => {log::error!("Error was found while flushing to stream. {}", e); break},
            }
            return;
        }
        if pair_session.encrypted {
            let mut key1 = [0u8; 32];
            Hkdf::<Sha512>::new(Some(b"Control-Salt"), &pair_session.shared_secret)
            .expand(b"Control-Write-Encryption-Key", &mut key1)
            .expect("HKDF expand failed");
            
            // data layout: [2-byte header][encrypted body][16-byte tag]
            let decrypted = decrypt_chunk(
                &key1,
                &data[0..2],                       // header as AAD
                &data[2..data.len() - 16],         // encrypted body
                &data[data.len() - 16..],          // auth tag
                &mut pair_session.read_counter
            );
            
            // Replace original buffer if needed
            data = decrypted;
        }
        println!("{}", String::from_utf8_lossy(&data));
        // Generate HttpRequest
        let mut request_data = http::generate_httprequest(&mut data);
        // Define EAT (encrypt after this) and response data
        let mut response_data: HttpResponse;
        let mut eat=false;
        // Start checking paths
        if request_data.method == "SETUP" {
            // Handle SETUP request
            response_data = handle_setup(&mut request_data, &mut pair_session, &airplay_session);
        } else if request_data.method == "RECORD" {
            // Handle RECORD request
            response_data = handle_record(request_data);
        } else {
            match request_data.path.as_str() {
                // Handle /pair-setup request
                "/pair-setup" => {
                    response_data = handle_pair_setup(request_data, &mut pair_session)
                },
                
                // Handle /pair-verify request
                "/pair-verify" => {
                    response_data = handle_pair_verify(request_data, &mut pair_session);
                    if pair_session.encrypted == true {
                        eat=true;
                    }
                },
                
                // Handle /auth-setup request
                "/auth-setup" => {
                    response_data = handle_auth_setup(request_data, &mut pair_session)
                },
                
                "/info" => {
                    response_data = handle_info(request_data, &mut pair_session)
                },

                /*"/feedback" => {
                    response_data = handle_feedback(request_data, Arc::clone(&airplay_session))
                },*/
                
                // Handle anything else
                _ => response_data = HttpResponse { status_code: String::from("200 OK"), protocol: request_data.protocol, headers: vec![], body: vec![], content_type: String::from(""), end: true },
            }
        }
        
        //let response = format!("RTSP/1.0 200 OK\r\nContent-Length: {}\r\nContent-Type: application/pairing+tlv8\r\nServer: AirTunes/366.0\r\nCSeq: 0\r\n\r\n", response_data.body.len());
        // this code might be terrible so fix it as you wish
        let mut response = format!("{} {}\r\n", response_data.protocol, response_data.status_code);
        if response_data.body.len() == 0 {
            response = format!("{}Server: {}\r\n\r\n", response, SERVER_NAME)
        } else {
            if pair_session.encrypted == true {
                response = format!("{}Content-Length: {}\r\nContent-Type: {}\r\nServer: {}\r\n", response, response_data.body.len(), String::from("application/ocet-stream"), SERVER_NAME);
            } else {
                response = format!("{}Content-Length: {}\r\nContent-Type: {}\r\nServer: {}\r\n", response, response_data.body.len(), response_data.content_type, SERVER_NAME);
            }
            for header in response_data.headers {
                response = format!("{}{}\r\n", response, header);
            }
            response = format!("{}\r\n", response);
        }
        
        let mut resp_buf: Vec<u8> = Vec::new();
        resp_buf.extend_from_slice(&response.as_bytes());
        resp_buf.extend_from_slice(&response_data.body);
        if !eat {
            if pair_session.encrypted == true {
                let mut key2: [u8; 32] = [0; 32];
                Hkdf::<Sha512>::new(Some(b"Control-Salt"), &pair_session.shared_secret)
                .expand(b"Control-Read-Encryption-Key", &mut key2)
                .expect("Error found in HKDF process");
                let (aad, chunk, auth_tag) =
                encrypt_chunk(&key2, &resp_buf, &mut pair_session.write_counter).expect("[FreakBob] calls again");
                resp_buf.clear();
                resp_buf.extend_from_slice(&aad);
                resp_buf.extend_from_slice(&chunk);
                resp_buf.extend_from_slice(&auth_tag);
            }
        }
        // flush response to client
        match stream.write_all(resp_buf.as_slice()) {
            Ok(_) => {},
            Err(e) => {log::error!("Error was found while writing header to stream. {}", e); break},
        }
        
        match stream.flush() {
            Ok(_) => {},
            Err(e) => {log::error!("Error was found while flushing to stream. {}", e); break},
        }
        if response_data.end == true {
            //break
        }
    }
}

pub fn init() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:7000")?;
    let pool = ThreadPool::new(4);
    log::info!("Initialized HTTP/RSTP server");
    // accept connections and process them serially
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        
        pool.execute(|| {
            handle_client(stream);
        });
    }
    log::debug!("ENDED!!!!!!!!!!");
    Ok(())
}