use std::thread;
use std::sync::{Arc, Mutex};
use mdns_sd::{ResolvedService, ServiceDaemon};
use env_logger::Builder;
use log::LevelFilter;
use std::time::Duration;
use std::thread::sleep;
mod srp;
mod networking;
mod parsing;
mod mfi;
mod persister;
mod ssl;
mod ctr;
mod airplay;
mod timing;
mod cc20p1305;
mod chachacryptor;
mod avcc;
mod HID;
#[cfg(any(target_os = "linux", target_os = "android"))]
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError, LinuxI2CMessage};

use crate::mfi::cp2::ret_cp_pubkey;

fn post_coprocessor(dev: &str) {
    log::info!("Begin Authentication Coprocessor POST sequence");
    let mut opt: Option<LinuxI2CDevice> = match LinuxI2CDevice::new(dev, 0x10) {
        Ok(lid) => {
            let toread: Arc<Mutex<LinuxI2CDevice>> = Arc::new(Mutex::new(lid));
            let dev_version = mfi::cp2::read_key(Some(toread.clone()), String::from("device_version"));
            let firm_version = mfi::cp2::read_key(Some(toread.clone()), String::from("firmware_version"));
            let pretty_version = match dev_version[0] {
                0x05 => "2.0C",
                0x07 => "3.0",
                _ => "(unknown version)"
            };
            log::info!("Connected to Authentication Coprocessor {}!\nFirmware Version: {:?}", pretty_version, firm_version[0]);
            let cert = ret_cp_pubkey(Some(toread));
            log::info!("Coprocessor Certificate: {:?}", cert);
            let chal = mfi::cp2::rsa_sign(Some(toread), "12211213131231231231".as_bytes());
            log::info!("Coprocessor Challenge Test Result: {:?}", chal.unwrap());
            Some(lid)
        },
        Err(err) => {
            log::warn!("Error while connecting to Authentication Coprocessor @ {}:\n{:?}", dev, err);
            log::warn!("Please confirm you have a Coprocessor 2.0 (3959) or a Coprocessor 3.0 (0177), if you do not have either, use fake_mfi.");
            None
        },
    };
}

fn main() {
    // signature, code is not needed at all
    println!(r#"{} _____       _        _           _   
    /  __ \     | |      | |         | |  
    | /  \/ __ _| |_ __ _| |_   _ ___| |_ 
    | |    / _` | __/ _` | | | | / __| __|
    | \__/\ (_| | || (_| | | |_| \__ \ |_ 
        \____/\__,_|\__\__,_|_|\__, |___/\__|
        __/ |        
        |___/
        
        {}Welcome to the {}rust-carplay Server{}!"#, "\x1b[31;1m", "\x1b[0m", "\x1b[37;1m", "\x1b[0m");
        
        env_logger::init();
        
        // Initialize configuration paths
        persister::init_config();
        
        post_coprocessor("/dev/i2c-2");
        
        // Initialize mdns service
        let mdns = Arc::new(ServiceDaemon::new().expect("Failed to create daemon"));
        
        // Run services in threads
        thread::Builder::new()
        .name("main-http-server".to_string()) // Set the desired name for the thread
        .spawn(|| {
            // This is the code that will run in the new thread
            networking::http_server::init();
        })
        .expect("Failed to spawn thread");
        let mdns1 = Arc::clone(&mdns);
        thread::spawn(move || {
            networking::bonjour_utils::advertise(mdns1);
        });
        let mdns2 = Arc::clone(&mdns);
        thread::spawn(move || {
            networking::bonjour_utils::discover(mdns2, start_session);
        });
        
        // Loop to keep the program busy
        // and prevent exiting
        loop {sleep(Duration::from_secs(1));};
    }
    
    fn start_session(info: Box<ResolvedService>) {
        let mut ip: Option<String> = None;
        
        for addr in info.addresses.iter() {
            if addr.is_ipv4() {
                let mut s = addr.to_string();
                s.push_str(":");
                s.push_str(&info.port.to_string());
                log::info!(" Address: {addr}");
                ip = Some(s);
                break; // stop after the first IPv4
            } else {
                log::warn!(" IPv6 detected! This will be ignored and instead an IPv4 will be searched (IPv6 support not implemented)");
                log::info!("  Address: {addr}");
            }
        }
        
        for prop in info.txt_properties.iter() {
            log::info!(" Property: {}", prop);
        }
        
        if let Some(ip) = ip {
            networking::http_client::control_command(ip, info.host.to_string());
        }
    }
    