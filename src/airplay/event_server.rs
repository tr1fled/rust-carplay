use std::{sync::{Arc, Mutex, mpsc}, time::Duration};
use std::io::{Read, Write, ErrorKind::WouldBlock};
use std::net::{TcpListener, TcpStream};
use threadpool::ThreadPool;
use std::thread;
use crate::{airplay::airplay_session::AirplaySession, networking::http_responders::pair_setup::Session};
use crate::networking::http_server::{decrypt_chunk, encrypt_chunk};
use sha2::{Sha512};
use hkdf::Hkdf;

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

pub fn handle_client(mut stream: TcpStream, shared_secret: Vec<u8>, airplay_session: Arc<Mutex<AirplaySession>>) {
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
        let read: usize;
        match stream.read(&mut buffer) {
            Ok(buf) => {
                // this would be done on a normal server, but there is no relevant response given, so nope
                /*read=buf;
                data.append(&mut buffer[..read].to_vec());
                if data.len() == 0 {
                } else {
                }*/
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