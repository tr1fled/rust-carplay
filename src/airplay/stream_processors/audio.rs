// audio server
// honestly this looks so hard to do on the original code
// but rlly what they call a 'node' is a queued audio packet

// so thats that
use std::io::Read;
use std::net::UdpSocket;
use std::os::raw::c_void;
use std::thread;
use sha2::Sha512;
use hkdf::Hkdf;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::process::{Command, Stdio};
use std::io::prelude::*; // Import the Write trait
use crate::airplay::airplay_session::{AirplaySession, AudioStream};
use crate::cc20p1305::bindings::{chacha20_poly1305_add_aad, chacha20_poly1305_decrypt, chacha20_poly1305_init_64x64, chacha20_poly1305_state, chacha20_poly1305_verify};
use crate::networking::http_server::decrypt_chunk;
use crate::networking::http_responders::pair_setup::chacha20_poly1305_decrypt_all_64x64_nopad;
use crate::chachacryptor::bindings::chachadecrypt_64x64;
use crate::airplay::audio::audio_format_to_asbd;
use crate::airplay::sinks::audio_gst::init_sink;
use gst_app::AppSrc;
use std::fs;
use std::io;

// NO -- debes ir sola, eres joven, eres rica. ahh
// - Jonaz
#[derive(Clone, Debug)]
pub struct RTPHeader {
    v_p_x_cc: u8,
    m_pt: u8,
    seq: u16,
    ts: u32,
    ssrc: u32
}
#[derive(Clone, Debug)]
pub struct RTPPacket {
    header: RTPHeader,
    payload: Vec<u8>,
    
}

impl RTPPacket {
    pub fn new(in_packet: &Vec<u8>) -> Self {
        //println!("INPKT LEN: {:?}", in_packet.clone().len());
        let in_header = in_packet.clone()[0..12].to_vec();
        Self {
            header: RTPHeader {
                v_p_x_cc: in_header[0],
                m_pt: in_header[1],
                seq: u16::from_be_bytes([in_header[2], in_header[3]]),
                ts: u32::from_be_bytes([in_header[4], in_header[5], in_header[6], in_header[7]]),
                ssrc: u32::from_be_bytes([in_header[8], in_header[9], in_header[10], in_header[11]])
            },
            payload: in_packet[12..].to_vec()
        }
    }
    // we are doing the same thing apple is doing, what is that? 9999 functions for everything
    // but i want to make it look as similar as possible, tho this might be even cleaner
    pub fn generate_aad(in_packet: &Self) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::new();
        result.extend_from_slice(&in_packet.header.ts.to_be_bytes());
        result.extend_from_slice(&in_packet.header.ssrc.to_be_bytes());
        result
    }
}

pub fn decrypt_frame_fresh(
    key: &[u8; 32],
    nonce: &[u8; 8],
    aad: &[u8],
    frame: &mut [u8],
) -> Result<usize, i32> {
    unsafe {
        let mut state = std::mem::MaybeUninit::<chacha20_poly1305_state>::uninit();
        
        // Init cipher with 8-byte nonce
        let body_len = frame.len().checked_sub(24).ok_or(-1)?;
        chacha20_poly1305_init_64x64(state.as_mut_ptr(), key.as_ptr(), frame[body_len+16..].as_ptr());
        
        // Header as AAD
        chacha20_poly1305_add_aad(state.as_mut_ptr(), aad.as_ptr() as *const c_void, aad.len());
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

fn roundup(value: u64, multiple: u64) -> u64 {
    ( ( ( (value) + ( (multiple) - 1 ) ) / (multiple) ) * (multiple) )
}

pub fn init_rtcp(rtcp_port_local: String, rtcp_address_remote: String) {
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", rtcp_port_local))
    .expect("couldn't bind to address");
    println!("res of socket conn: {:?}", socket.connect(rtcp_address_remote));
}

/// Start the TCP listener and spawn threads
pub fn init(airplay_session: Arc<Mutex<AirplaySession>>, shared_secret: Vec<u8>, data_port: String, rtcp_port_local: String, rtcp_port_remote: String, stream: Arc<Mutex<AudioStream>>) {
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", data_port)).expect("Failed to");
    init_sink(Arc::clone(&stream));
    let pipeline: gst::Pipeline;
    let appsrc: gst_app::AppSrc;
    let sci: String;
    {
        let st = stream.lock().unwrap();
        pipeline = st.pipeline.as_ref().unwrap().clone();
        appsrc = st.sink.as_ref().unwrap().clone();
        sci = st.stream_connection_id.clone();
    }
    let peer_ip: String;
    {
        let aps = airplay_session.lock().unwrap();
        peer_ip = aps.peer_ip.clone();
    }
    thread::spawn(move || {
        init_rtcp(rtcp_port_local, format!("{}:{}", peer_ip, rtcp_port_remote));
    });
    let fmt: u64;
    let spf: u32;
    {
        let stdat = stream.lock().unwrap();
        fmt = stdat.audio_format;
        spf = stdat.spf.try_into().unwrap();
    }
    let asbd = audio_format_to_asbd(fmt);
    // holy trying hell
    let bytes_per_unit = match asbd.bytes_per_frame {
        0 => { ((TryInto::<u32>::try_into(roundup(asbd.bits_per_channel.into(), 8)).unwrap() * asbd.channels_per_frame) / 8) },
        _ => asbd.bytes_per_frame
    };
    // equation
    let i = ( spf + ( (spf / 441) + 16 ) ) * asbd.bytes_per_frame;
    // Bind socket to port 7000
    let mut buf = [0; 1024]; // A buffer to store incoming data
    let mut key = [0u8; 32];
    Hkdf::<Sha512>::new(Some(format!("DataStream-Salt{}", sci).as_bytes()), &shared_secret)
    .expand(b"DataStream-Output-Encryption-Key", &mut key)
    .expect("HKDF expand failed");
    let mut read_counter: u64 = 0;
    loop {
        // recv_from returns the number of bytes read and the address of the sender.
        let (len, addr) = socket.recv_from(&mut buf).expect("failed to recv");
        //println!("received the following data: {:02X?}", &buf[..len]);
        let mut audio_data = RTPPacket::new(&buf[..len].to_vec());
        let aad = RTPPacket::generate_aad(&audio_data);
        //println!("THE LENGTH OF THE AUDIO DATA PAYLOAD IS: {:?}", audio_data.payload.len());
        //println!("TS: {}", &audio_data.header.ts);
        let mut frame = audio_data.payload.clone();
        unsafe {
            let mut state = std::mem::MaybeUninit::<chacha20_poly1305_state>::uninit();
            
            // Init cipher with 8-byte nonce
            let body_len = frame.len().checked_sub(24).ok_or(-1).unwrap();
            chacha20_poly1305_init_64x64(state.as_mut_ptr(), key.as_ptr(), frame[body_len+16..].as_ptr());
            
            // Header as AAD
            chacha20_poly1305_add_aad(state.as_mut_ptr(), aad.as_ptr() as *const c_void, aad.len());
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
                eprintln!("OSStatus {} while trying to decrypt", err);
            }else {
                //file.write_all(&frame[..body_len].to_vec()).unwrap();
                //println!("{:02X?}", frame[..body_len].to_vec());
                let mut buf = gst::Buffer::from_slice(frame[..body_len].to_vec());
                let mut nbuf = buf.get_mut().unwrap();
                let pts_value = gst::ClockTime::from_seconds((1024 / 44100).try_into().unwrap());
                //let pts_value1 = gst::ClockTime::from_mseconds((audio_data.header.ts / 44100).try_into().unwrap());
                nbuf.set_duration(Some(pts_value));
                //nbuf.set_pts(Some(pts_value1));
                appsrc.push_buffer(buf).unwrap();
            }
        }
        read_counter += 1;
        //println!("audio data: {:02X?}", &audio_data.payload[0..8]);
    }
}