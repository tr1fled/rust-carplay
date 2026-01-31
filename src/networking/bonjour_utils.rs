use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::sync::Arc;
use log;
use uuid::Uuid;
use crate::persister::{self, CFG_PAIRING_IDENTITY};

pub fn discover(mdns: Arc<ServiceDaemon>, callback: fn(Box<ResolvedService>)) {
    let receiver = mdns.browse("_carplay-ctrl._tcp.local.").expect("Failed to browse");
    log::info!("Started discovering _carplay-ctrl._tcp.local. with mDNS");
    while let Ok(event) = receiver.recv() {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                log::info!("Resolved a new service: {}\n host: {}\n port: {}", info.fullname, info.host, info.port);
                callback(info);
            }
            other_event => {
                log::debug!("{:?}", other_event);
            }
        }
    }
}

pub fn advertise(mdns: Arc<ServiceDaemon>) {
    // Create a service info.
    log::trace!("Registering Airplay server via mDNS");
    let service_type = "_airplay._tcp.local.";
    let instance_name = "cp";
    let ip = "192.168.42.1";
    let service_hostname = "192.168.42.1.local.";
    let port = 7000;
    let pi: String;
    match persister::read_key(CFG_PAIRING_IDENTITY) {
        Ok(pid) => pi = String::from_utf8_lossy(&pid).to_string(),
        Err(e) => {
            panic!("Error {} has been found while trying to fetch the pairing identity, this is a fatal error! While the program can go on, it is a good practice to fix this.", e);
        }
    }
    let properties = [
        ("deviceid", "DC:A6:32:C3:F3:DE"),
        ("features", "0x4040280,0x61"),
        //("features", "0x4040280,0x21"),
        ("flags", "0x29a404"),
        ("model", "Catalyst1,1"),
        ("pi", pi.as_str()),
        ("srcvers", "320.17")
    ];

    let my_service = ServiceInfo::new(
        service_type,
        instance_name,
        service_hostname,
        ip,
        port,
        &properties[..],
    )
    .expect("valid service info")
    .enable_addr_auto();

    // Register with the daemon, which publishes the service.
    mdns.register(my_service)
        .expect("Failed to register our service");

    log::info!("Registered service {}.{}", &instance_name, &service_type);
}