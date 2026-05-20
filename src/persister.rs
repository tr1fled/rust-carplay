use dirs;
use std::path::PathBuf;
use std::fs;
use log;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::io; // Import prelude for write_all and other I/O traits
use uuid::Uuid;
use rand::rngs::OsRng;
use ed25519_dalek::SigningKey;
use ed25519_dalek::Signature;

const CONFIG_PATH: &str = ".local/share/catalyst-carplay";

pub const CFG_ED25519_PK: u8 =          0x0;
pub const CFG_ED25519_SK: u8 =          0x1;
pub const CFG_PAIRING_IDENTITY: u8 =    0x2;

const ED25519_PK_PATH: &str = "ed25519/host_pk.bin";
const ED25519_SK_PATH: &str = "ed25519/host_sk.bin";
const PAIRING_IDENTITY_PATH: &str = "pairing_identity.txt";
const PEER_DATA_PATH: &str = "peers";

fn get_home_directory() -> PathBuf {
    match dirs::home_dir() {
        Some(path) => {
            path
        }
        None => {
            panic!("Could not determine the home directory, while the program can go on and generate live values, it is a good practice to modify the code in the 'persister'.");
        }
    }
}

/// Initializes persistent configuration data structure
pub fn init_config() {
    let path = get_home_directory();
    let my_file_path = path.join(CONFIG_PATH);
    if !my_file_path.exists() {
        // Create the Ed25519 data structure
        fs::create_dir_all(&my_file_path.join("ed25519")).expect("Error while creating basic config paths");
        fs::create_dir_all(&my_file_path.join(PEER_DATA_PATH)).expect("Error while creating basic config paths");
        File::create(&my_file_path.join(ED25519_PK_PATH)).expect("Error while creating Ed25519 key binaries");
        File::create(&my_file_path.join(ED25519_SK_PATH)).expect("Error while creating Ed25519 key binaries");

        // Ed25519 keypair generation
        log::info!("Generating & saving new Ed25519 keypair");
        let mut csprng = OsRng;
        let signing_key: SigningKey = SigningKey::generate(&mut csprng);
        let mut ed25519_pk = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false) // This is the key for truncating
        .open(&my_file_path.join(ED25519_PK_PATH)).expect("Issue while opening Ed25519 PK");
        ed25519_pk.write_all(signing_key.verifying_key().as_bytes()).expect("Issue while writing to the Ed25519 PK");
        let mut ed25519_sk = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false) // This is the key for truncating
        .open(&my_file_path.join(ED25519_SK_PATH)).expect("Issue while opening Ed25519 SK");
        ed25519_sk.write_all(signing_key.as_bytes()).expect("Issue while writing to the Ed25519 SK");
        
        // Create pairing identity
        log::info!("Generating & saving new pairing_identity");
        let mut file = File::create(my_file_path.join(PAIRING_IDENTITY_PATH)).expect(format!("Error while creating the {} file", PAIRING_IDENTITY_PATH).as_str());
        file.write_all(Uuid::new_v4().to_string().as_bytes()).expect("Error while writing pairing identity"); 
    }
}

/// Reads a configuration key from persistent storage
///
/// # Arguments
/// * `key` - Key to read, defined as a const by persister.rs (ex. `CFG_ED25519_PK`)
///
/// # Returns
/// `Result<Vec<u8>>` or `Err`
pub fn read_key(key: u8) -> std::io::Result<Vec<u8>> {
    let path = get_home_directory().join(CONFIG_PATH);
    if key == CFG_ED25519_PK {
        fs::read(&path.join(ED25519_PK_PATH))
    } else if key == CFG_ED25519_SK {
        fs::read(&path.join(ED25519_SK_PATH))
    } else if key == CFG_PAIRING_IDENTITY {
        fs::read(&path.join(PAIRING_IDENTITY_PATH))
    } else {
        log::error!("Tried to read from invalid key. Please review your code.");
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Tried to read from invalid key. Please review your code.",
        ))
    }
}

/// Writes a configuration key to persistent storage
///
/// # Arguments
/// * `key` - Key to write, defined as a const by persister.rs (ex. `CFG_ED25519_PK`)
/// * `data` - Data to write. Please format it as Vec<u8>.
/// * `truncate` - To truncate or not to truncate existing file content. Bool.
///
/// # Returns
/// `Result` or `Err`
pub fn write_key(key: u8, data: Vec<u8>, truncate: bool) -> std::io::Result<()> {
    let path = get_home_directory().join(CONFIG_PATH);
    if key == CFG_ED25519_PK {
        let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(truncate) // This is the key for truncating
        .open(&path.join(ED25519_PK_PATH))?;
        file.write_all(&data)?;
        Ok(())
    } else if key == CFG_ED25519_SK {
        let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(truncate) // This is the key for truncating
        .open(&path.join(ED25519_SK_PATH))?;
        file.write_all(&data)?;
        Ok(())
    } else if key == CFG_PAIRING_IDENTITY {
        let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(truncate) // This is the key for truncating
        .open(&path.join(PAIRING_IDENTITY_PATH))?;
        file.write_all(&data)?;
        Ok(())
    } else {
        log::error!("Tried to read from invalid key. Please review your code.");
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Tried to write to invalid key. Please review your code.",
        ))
    }
}

/// Reads peer data from persistent storage
/// # Arguments
/// * `peer_id` - Peer's ID which will be queried on disk
/// # Returns
/// `Result<Vec<u8>>` or `Err`
pub fn read_peer_data(peer_id: Vec<u8>) -> Vec<u8> {
    let path = get_home_directory().join(PEER_DATA_PATH);
    match fs::read(&path.join(String::from_utf8(peer_id.clone()).expect("Invalid UTF8 sequence")).join("ed_pk.bin")) {
        Ok(buf) => buf,
        Err(e) => panic!("While trying to read the data of peer {}, an error ocurred: {}", String::from_utf8(peer_id).expect("Invalid UTF8 sequence"), e),
    }
}

pub fn write_peer_data(peer_id: Vec<u8>, peer_ed_pk: Vec<u8>) {
    let path = get_home_directory().join(PEER_DATA_PATH).join(String::from_utf8(peer_id.clone()).expect("Invalid UTF8 sequence"));
    if !path.exists() {
        fs::create_dir_all(&path).expect("Error while creating peer config path");
    }
    let mut peer_pk = OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true) // This is the key for truncating
    .open(&path.join("ed_pk.bin")).expect("Issue while opening peer's Ed25519 PK on disk");
    peer_pk.write_all(peer_ed_pk.as_slice()).expect("Issue while writing to the peer's Ed25519 PK");
}
