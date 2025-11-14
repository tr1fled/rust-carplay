use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::os::raw::c_void;
use threadpool::ThreadPool;
use sha2::Sha512;
use hkdf::Hkdf;
use std::fs::File;
use std::io::prelude::*; // Import the Write trait
use crate::airplay::airplay_session::Stream;
use crate::networking::http_responders::pair_setup::chacha20_poly1305_decrypt_all_64x64_nopad;
use crate::chachacryptor::bindings::chachadecrypt_64x64;
use crate::cc20p1305::bindings::{
    chacha20_poly1305_init_64x64,
    chacha20_poly1305_add_aad,
    chacha20_poly1305_decrypt,
    chacha20_poly1305_verify,
    chacha20_poly1305_state,
};
use crate::avcc::avcc::{
    avcc_to_annexb,
    annexb_process_data
};

#[derive(Debug, Clone)]
pub struct ScreenSession {
    pub salt: [u8; 16],
    pub verifier: Vec<u8>,
    pub b: Vec<u8>,        // was [u8;64]
    pub b_pub: Vec<u8>,
    pub shared_secret: Vec<u8>,
    pub encrypt_key: [u8; 32],
    pub mfi_encrypt_key: [u8; 32],
    // peer data
    pub peer_identifier: Vec<u8>,
    pub peer_ed_pk: Vec<u8>,
    // host's ed
    pub host_ed_pk: Vec<u8>,
    pub host_ed_sk: Vec<u8>,
    // host's curve
    pub host_curve_sk: Vec<u8>,
    pub host_curve_pk: Vec<u8>,
    // peer's curve
    pub peer_curve_pk: Vec<u8>,
    pub encrypted : bool,
    pub read_counter: u64,
    pub write_counter: u64,
}

impl ScreenSession {
    pub fn new() -> Self {
        Self {
            salt: [0; 16],
            verifier: Vec::new(),
            b: Vec::new(),
            b_pub: Vec::new(),
            shared_secret: Vec::new(),
            encrypt_key: [0; 32],
            mfi_encrypt_key: [0; 32],
            peer_identifier: Vec::new(),
            peer_ed_pk: Vec::new(),
            host_ed_pk: Vec::new(),
            host_ed_sk: Vec::with_capacity(32),
            host_curve_sk: Vec::new(),
            host_curve_pk: Vec::new(),
            peer_curve_pk: Vec::new(),
            encrypted: false,
            read_counter: 0,
            write_counter: 0,
        }
    }
}

/// Increment 8-byte little endian nonce exactly like C's LittleEndianIntegerIncrement
fn increment_nonce_le(nonce: &mut [u8; 8]) {
    let mut carry = 1u8;
    for byte in nonce.iter_mut() {
        let (new_byte, overflow) = byte.overflowing_add(carry);
        *byte = new_byte;
        if !overflow { break; }
        carry = 1;
    }
}

/// Decrypt a single frame with ChaCha20-Poly1305
pub fn decrypt_frame_fresh(
    key: &[u8; 32],
    nonce: &[u8; 8],
    header: &[u8],
    frame: &mut [u8],
) -> Result<usize, i32> {
    unsafe {
        let mut state = std::mem::MaybeUninit::<chacha20_poly1305_state>::uninit();
        
        // Init cipher with 8-byte nonce
        chacha20_poly1305_init_64x64(state.as_mut_ptr(), key.as_ptr(), nonce.as_ptr());
        
        // Header as AAD
        chacha20_poly1305_add_aad(state.as_mut_ptr(), header.as_ptr() as *const c_void, header.len());
        
        let body_len = frame.len().checked_sub(16).ok_or(-1)?;
        let decrypted_len = chacha20_poly1305_decrypt(
            state.as_mut_ptr(),
            frame.as_ptr() as *const c_void,
            body_len,
            frame.as_mut_ptr() as *mut c_void,
        );
        
        let mut err: i32 = 0;
        chacha20_poly1305_verify(
            state.as_mut_ptr(),
            frame[decrypted_len..].as_mut_ptr() as *mut c_void,
            frame[body_len..body_len + 16].as_ptr(),
            &mut err,
        );
        
        if err != 0 {
            return Err(err);
        }
        
        Ok(body_len)
    }
}

/// Convert any signed integer to a u64 string for HKDF salt
fn sci_to_u64_string(sci_str: &str) -> String {
    if let Ok(val) = sci_str.parse::<i64>() {
        format!("{}", val.unsigned_abs())
    } else {
        sci_str.to_string()
    }
}

/// Handle a single TCP client
fn handle_client(mut stream: TcpStream, shared_secret: Vec<u8>, sci: String) {
    println!("CAUGHT WITHIN AIRPLAY_SESSION");
    
    let mut read_counter: u64 = 0;
    let mut nonce: [u8; 8] = [0; 8];
    let mut file = File::create("/home/poc/Desktop/rustcp/carplay.h264").expect("fail to c path");
    loop {
        println!("caught in the loop! [screen]");
        let mut header_data = [0; 128];
        let read = match stream.read(&mut header_data) {
            Ok(r) if r > 0 => r,
            Ok(_) => continue,
            Err(e) => {
                log::error!("Error reading buffer: {}", e);
                return;
            }
        };
        println!("caught in the afterread! [screen]");
        if read < 128 { continue; }
        let data_length = u32::from_le_bytes(header_data[0..4].try_into().expect("Wtf? Buffer that will go 0..4 is untryable...")) as usize;
        let mut body_data: Vec<u8> = vec![0; data_length];
        if let Err(e) = stream.read_exact(&mut body_data) {
            panic!("Error reading body: {}", e);
        }    
        
        let mut key = [0u8; 32];
        let salt_string = sci_to_u64_string(&sci);
        Hkdf::<Sha512>::new(Some(format!("DataStream-Salt{}", salt_string).as_bytes()), &shared_secret)
        .expand(b"DataStream-Output-Encryption-Key", &mut key)
        .expect("HKDF expand failed");
        
        // Set nonce from read_counter (LE bytes)
        let body_len = u32::from_le_bytes(header_data[0..4].try_into().unwrap());
        println!("nonce: {:x?}", nonce);
        println!("length: {:x?}",  &header_data[0..4]);
        println!("header_data: {:x?}", &header_data[..16]); // first 16 bytes for sanity
        //println!("body_data (first 16 bytes): {:x?}", &body_data[..16]);
        let mut state_uninit = std::mem::MaybeUninit::<chacha20_poly1305_state>::uninit();
        let state_ptr = state_uninit.as_mut_ptr();
        if &header_data[4] == &0u8 {
            // you keep me waiting, down here, so far below
            //println!("framedata: {:x?}", chacha20_poly1305_decrypt_all_64x64_nopad(&key, &nonce, &body_data, Some(&header_data)));
            let body_len = u32::from_le_bytes(header_data[0..4].try_into().unwrap());
            let body_len_usize = body_len as usize;
            
            // Ensure full body is available
            if body_data.len() < body_len_usize {
                log::error!(
                    "Truncated frame: header says {} bytes, but body_data.len() = {}",
                    body_len_usize,
                    body_data.len()
                );
                continue; // don't call C FFI
            }
            
            // Call the C decryption function
            unsafe {
                let ret = chachadecrypt_64x64(
                    key.as_ptr(),
                    nonce.as_mut_ptr(),
                    header_data.as_ptr() as *const c_void,
                    body_data.as_mut_ptr() as *mut c_void,
                    body_len,
                );
                
                if ret != 0 {
                    log::error!("Decryption failed: OSStatus = {}", ret);
                    continue; // or handle error
                } else {
                    println!("Screen frame decrytion successful!");
                }
                if read_counter == 1  {
                    println!("First video data: {:x?}", body_data);
                } else if read_counter == 2  {
                    println!("Second video data: {:x?}", body_data);
                } else if read_counter == 3  {
                    println!("Third video data: {:x?}", body_data);
                }
                file.write_all(&annexb_process_data(body_data).as_slice()).expect("son💀💀💀💀we cant process that frame");
            }
            //increment_nonce_le  (&mut nonce);
            read_counter += 1;
        } else if &header_data[4] == &1u8 {
            // TO-DO :: Write everything to an interpretor or to filesystem.
            // Will do filesystem then attempt to start an Annex-B stream 
            file.write_all(&avcc_to_annexb(body_data).as_slice()).expect("son💀💀💀💀we cant process that");
        }
        /*unsafe {
        chacha20_poly1305_init_64x64(state_ptr, key.as_ptr(), nonce.as_ptr());
        chacha20_poly1305_add_aad(state_ptr, header_data.as_ptr() as *const c_void, header_data.len());
        
        let mut length: [u8; 4] = header_data[0..4].try_into().expect("Failed to try into length");
        println!("dec");
        let body_len = u32::from_le_bytes(length) as usize;
        let mut len = chacha20_poly1305_decrypt(state_ptr, body_data.as_ptr() as *const c_void, body_len-16, body_data.as_mut_ptr() as *mut c_void);
        let mut err: i32 = 0;
        let mut my_num_ptr: *mut i32 = &mut err;
        println!("ver");
        len += chacha20_poly1305_verify(state_ptr, body_data[len..].as_mut_ptr() as *mut c_void, body_data[body_len-16..].as_ptr(), my_num_ptr);
        if err != 0 {
        println!("Fatal error: {}", err);
        }
        }*/
        
        // Increment nonce exactly like C
    }
}

/// Start the TCP listener and spawn threads
pub fn init(shared_secret: Vec<u8>, port: String, streamdata: Stream) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
    .expect("Failed to initialize the Screen Stream Receiver");
    let pool = ThreadPool::new(4);
    
    log::info!("Initialized Screen Stream Receiver");
    
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let ss = shared_secret.clone();
        let sci = streamdata.stream_connection_id.clone();
        
        pool.execute(move || {
            handle_client(stream, ss, sci);
        });
    }
}
