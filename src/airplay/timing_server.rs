// the VERY UGLY timing server
// why does apple have to go and make things so complicated?
// CPU cycle-accurate timer? come on now..... it cant be that hard apple..
// but i guess thats why they give us a plugin instead of making us recode it.
// but since apple's code is garbage to the naked eye, we will rewrite it
// even though we have never done this before

// what do you think, could this code be tim apple certified©️?

use crate::timing::bindings::{AirTunesClockRef, AirTunesClock_GetSynchronizedTime, AirTunesTime};
use crate::{airplay::airplay_session::AirplaySession, networking::http_responders::pair_setup::Session, timing::bindings};
use std::sync::{Arc, Mutex};
use std::net::UdpSocket;
use std::io;
use log;
use std::thread;
use std::time::Duration;

// thanks timmy apple for your beautiful constant which i just stole
const kNTPvsUnixSeconds: u32 = 2208988800;

pub fn init(pair_session: Session, airplay_session: Arc<Mutex<AirplaySession>>, airtunes_clock: AirTunesClockRef) {
    println!("hello to the timing server");
    // Bind socket to port 7000
    let socket = UdpSocket::bind("0.0.0.0:7000").expect("Failed to");
    log::info!("UDP timing socket opened on: 0.0.0.0:7000");
    println!("opened");
    let mut ip = String::new();
    {
        // copy the data to read without further locking
        let mut aps = airplay_session.lock().unwrap();
        aps.peer_ip.clone_into(&mut ip);
        println!("THY PORT WHERE WE WILL BE SENDING THEE DATA IS AS FOLLOWS: {}", &aps.peer_timing_port.to_string());
        ip = format!("{}:{}", ip, aps.peer_timing_port.to_string());
        println!("THY HOST WHERE WE WILL BE SENDING THEE DATA IS AS FOLLOWS: {}", &ip);
    }

    // Set up buffer & AirTunes time
    let mut buf: [u8; 1024] = [0; 1024]; // 32-byte buffer
    let mut at_now = AirTunesTime { secs: 0, frac: 0 }; // AirTunesTime new struct
    let mut now: *mut AirTunesTime = &mut at_now; // Convert to raw mutable pointer
    let mut rtcpTILastTransmitTimeHi: i32 = 0;
    let mut rtcpTILastTransmitTimeLo: u64 = 0;
    
    loop {
        unsafe {
            AirTunesClock_GetSynchronizedTime(airtunes_clock, now);
            // do logic that timmy apple has made for us in the spec code
            // converted to BE because c code does it
            rtcpTILastTransmitTimeHi = (at_now.secs + kNTPvsUnixSeconds as i32);
            rtcpTILastTransmitTimeLo = (at_now.frac >> 32); // why did rust do this? (expected i64, found u64) BEATS ME!
        }
        let mut to_send: Vec<u8> = vec![
            0x80,
            0xd2,
            0x00, 0x07,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        println!("{:02X?}", &rtcpTILastTransmitTimeHi.to_be_bytes());
        println!("{:02X?}", &rtcpTILastTransmitTimeLo);
        to_send.extend_from_slice(&rtcpTILastTransmitTimeHi.to_be_bytes());
        to_send.extend_from_slice(&rtcpTILastTransmitTimeLo.to_be_bytes()[..4]);
        assert!(to_send.len() == 32);
        socket.send_to(&to_send, &ip).expect("Failed to send UDP data");
        let (amt, src) = socket.recv_from(&mut buf).expect("Failed to receive UDP data");
        let received_data = &buf[..amt]; // Slice the buffer to only include received data
        println!("We got the time back! it is: {:02X?}", received_data);
        thread::sleep(Duration::from_secs(2));
    }
}