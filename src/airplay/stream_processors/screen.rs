use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::os::raw::c_void;
use gst_app::AppSrc;
use log;
use threadpool::ThreadPool;
use sha2::Sha512;
use hkdf::Hkdf;
use std::io::{Write};
use std::sync::{Arc, Mutex};
use gst::Buffer;
use crate::airplay::airplay_session::{AirplaySession, Stream};
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
    avc_to_annexb, avcc_to_annexb
};

/// Convert any signed integer to a u64 string for HKDF salt
fn sci_to_u64_string(sci_str: &str) -> String {
    if let Ok(val) = sci_str.parse::<i64>() {
        format!("{}", val.unsigned_abs())
    } else {
        sci_str.to_string()
    }
}

/// Handle a single TCP client
fn handle_client(mut stream: TcpStream, shared_secret: Vec<u8>, sci: String, airplay_session: Arc<Mutex<AirplaySession>>) {
    let mut read_counter: u64 = 0;
    let mut nonce: [u8; 8] = [0; 8];
    let mut sinkdata: Arc<Mutex<crate::airplay::sinks::gst_gtk4::GstSinkData>>;
    {
        sinkdata = airplay_session.lock().unwrap().sink.as_ref().expect("Should be some, but is none? Fatal failure.").clone();
    }
    let mut ds: AppSrc =  {
        let sd = sinkdata.lock().unwrap();
        sd.data_sink.clone().expect("APPSRC IS NONE! FAIL!")
    };
    loop {
        let mut header_data = [0; 128];
        let read = match stream.read(&mut header_data) {
            Ok(r) if r > 0 => r,
            Ok(_) => {
                println!("continuing, there could be an error after this, yes? no? lmk");
                continue;
            }, // PIN, PLEASE DONT DO CONTINUE WITHOUT INCREASING BUF
            Err(e) => {
                log::error!("Error reading buffer: {}", e);
                return;
            }
        };
        //println!("caught in the afterread! [screen]");
        if read < 128 { continue; }
        let data_length = u32::from_le_bytes(header_data[0..4].try_into().unwrap()) as usize; // unwrap because this will never EVER fail
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
        let mut state_uninit = std::mem::MaybeUninit::<chacha20_poly1305_state>::uninit();
        let state_ptr = state_uninit.as_mut_ptr();
        if &header_data[4] == &0u8 {
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
                let ononce = nonce.clone();
                let ret = chachadecrypt_64x64(
                    key.as_ptr(),
                    nonce.as_mut_ptr(),
                    header_data.as_ptr() as *const c_void,
                    body_data.as_mut_ptr() as *mut c_void,
                    body_len,
                );
                if ononce == nonce {
                    println!("nonce recovery performed");
                    nonce = (u64::from_le_bytes(nonce)+1).to_le_bytes();
                }
                println!("{:02X?}", &nonce);
                
                if ret != 0 {
                    log::error!("Decryption failed: OSStatus = {}", ret);
                    continue; // or handle error
                }
                {   
                    // Now safe to push
                    let apd = avc_to_annexb(body_data);
                    let buf = Buffer::from_slice(apd);// used to be apd
                    match ds.push_buffer(buf) {
                        Ok(_) => (), // do nothing because all went as intended
                        Err(e) => {
                            log::error!("Error while pushing data to sink: {:?}", e)
                        },
                    }
                }
            }
            read_counter += 1;
        } else if &header_data[4] == &1u8 {
            let apd = avcc_to_annexb(body_data);
            let buf = Buffer::from_slice(apd);
            match ds.push_buffer(buf) {
                Ok(_) => (), // do nothing because all went as intended
                Err(e) => {
                    log::error!("Error while pushing data to sink: {:?}", e)
                },
            }
        }
    }
}

/// Start the TCP listener and spawn threads
pub fn init(shared_secret: Vec<u8>, port: String, stream_connection_id: String, airplay_session: Arc<Mutex<AirplaySession>>) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
    .expect("Failed to initialize the Screen Stream Receiver");
    let pool = ThreadPool::new(4);
    
    log::info!("Initialized Screen Stream Receiver");
    
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let ss = shared_secret.clone();
        let sci = stream_connection_id.clone();
        let aps = airplay_session.clone();
        pool.execute(move || {
            handle_client(stream, ss, sci, aps);
        });
    }
}
