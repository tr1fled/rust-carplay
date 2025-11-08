use std::thread;
use std::sync::Arc;
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
