// i hate to be a trendy fucker, but this time i will say it
// I HAVE NO IDEA HOW THIS IS NOT FALLING APART

use std::marker::PhantomData;
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[allow(dead_code)]
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::{os::raw::c_void, process::ExitStatus, ptr};
use std::sync::{Arc, Mutex};
use std::ffi::{CStr, CString};
use libc::size_t;
use log;
use rand::{rngs::OsRng, RngCore};
use sha2::{Sha512};
use sha1::{Digest, Sha1};
use hkdf::Hkdf;
use chacha20poly1305::{
    aead::{Aead, KeyInit, generic_array::GenericArray},
    ChaCha20Poly1305, // 96-bit nonce version (standard)
    XChaCha20Poly1305 // 192-bit nonce version
};
use chacha20poly1305::aead::Payload;
use ed25519_dalek::*;
use crate::parsing::tlv8::*;
use crate::parsing::http::{
    HttpRequest,
    HttpResponse
};
use crate::srp::bindings::*;
// change as you wish, since we do not have an mfi chip we will 'troll' apple
use crate::mfi::cp2;
use crate::mfi::cp2::read_key;
use crate::persister::{self, CFG_ED25519_PK, CFG_ED25519_SK, CFG_PAIRING_IDENTITY};
use num_bigint::BigUint;
use i2cdev::core::*;
#[cfg(any(target_os = "linux", target_os = "android"))]
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError, LinuxI2CMessage};


// thanks tim apple for the spacings and all

// SRP
const kPairSetupSRPUsername: &str =     "Pair-Setup";
const kPairSetupSRPPassword: &str =     "3939"; // Fixed PIN used by CarPlay, this would not be fixed in a normal HKP flow

// HKDF
const kPairSetupEncryptSaltPtr: &[u8] =         b"Pair-Setup-Encrypt-Salt";
const kPairSetupEncryptInfoPtr: &[u8] =         b"Pair-Setup-Encrypt-Info";
const kMFiPairSetupSaltPtr: &[u8] =             b"MFi-Pair-Setup-Salt";
const kMFiPairSetupInfoPtr: &[u8] =             b"MFi-Pair-Setup-Info";
const kPairSetupControllerSignSalt: &[u8] =		b"Pair-Setup-Controller-Sign-Salt";
const kPairSetupControllerSignInfo: &[u8] =		b"Pair-Setup-Controller-Sign-Info";
const kPairSetupAccessorySignSalt: &[u8] =	b"Pair-Setup-Accessory-Sign-Salt";
const kPairSetupAccessorySignInfo: &[u8] =	b"Pair-Setup-Accessory-Sign-Info";

const kPairSetupStateNone: u8 =         0; // Controller -> Accessory  -- Start Request
const kPairSetupStateM1: u8 =           1; // Controller -> Accessory  -- Start Request
const kPairSetupStateM2: u8 =           2; // Accessory  -> Controller -- Start Response
const kPairSetupStateM3: u8	=           3; // Controller -> Accessory  -- Verify Request
const kPairSetupStateM4: u8	=           4; // Accessory  -> Controller -- Verify Response
const kPairSetupStateM5: u8 =           5; // Controller -> Accessory  -- Exchange Request
const kPairSetupStateM6: u8 =           6; // Accessory  -> Controller -- Exchange Response
const kPairSetupStateDone: u8 =         7;

#[derive(Debug, Clone)]
pub struct Session {
    pub srp_ctx: *mut SRPPrivate_libsrp,
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
    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub i2c_device: Option<&mut LinuxI2CDevice>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            srp_ctx: std::ptr::null_mut(),
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
            #[cfg(any(target_os = "linux", target_os = "android"))]
            i2c_device: get_i2c_dev("/dev/i2c-1"),
        }
    }
}
unsafe impl Send for Session {}

//#[cfg(any(target_os = "linux", target_os = "android"))]
fn get_i2c_dev(dev: &str) -> Option<&mut LinuxI2CDevice> {
    let mut opt: Option<&mut LinuxI2CDevice>;
    match LinuxI2CDevice::new(dev, 0x10) {
        Ok(lid) => {
            opt = Some(&mut lid);
            let dev_version = read_key(&opt, String::from("device_version"));
            let firm_version = read_key(&opt, String::from("firmware_version"));
            let pretty_version = match dev_version[0] {
                0x05 => "2.0C",
                0x07 => "3.0",
                _ => "(unknown version)"
            };
            log::info!("Connected to Authentication Coprocessor {}!\nFirmware Version: {:?}", pretty_version);
        },
        Err(err) => {
            opt = None;
            log::warn!("Error while connecting to Authentication Coprocessor @ {}:\n{:?}", dev, err);
            log::warn!("Please confirm you have a Coprocessor 2.0 (3959) or a Coprocessor 3.0 (0177), if you do not have either, use fake_mfi.")
        },
    }
    opt
}

/// Encrypts data using 32-byte key + 8-byte nonce (like chacha20_poly1305_encrypt_all_64x64)
fn chacha20_poly1305_encrypt_all_64x64(
    key: &[u8; 32],
    nonce: &[u8; 8],
    plaintext: &[u8],
    aad: Option<&[u8]>,
) -> (Vec<u8>, [u8; 16]) {
    // ChaCha20Poly1305 usually expects a 12-byte nonce.
    // Apple’s variant (64x64) uses 8-byte nonces, so we’ll pad with 4 zero bytes.
    let mut nonce_padded = [0u8; 12];
    nonce_padded[4..].copy_from_slice(nonce);
    
    let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(key));
    
    let payload = Payload {
        msg: plaintext,
        aad: aad.unwrap_or(&[]),
    };
    
    let ciphertext_with_tag = cipher.encrypt(GenericArray::from_slice(&nonce_padded), payload)
    .expect("encryption failure");
    
    // Split ciphertext and tag
    let tag_index = ciphertext_with_tag.len() - 16;
    let (ciphertext, tag) = ciphertext_with_tag.split_at(tag_index);
    
    let mut tag_array = [0u8; 16];
    tag_array.copy_from_slice(tag);
    
    (ciphertext.to_vec(), tag_array)
}

pub fn chacha20_poly1305_decrypt_all_64x64(
    key: &[u8; 32],
    nonce: &[u8; 8],
    storage: &[u8], // ciphertext + tag, as received
    aad: Option<&[u8]>,
) -> Vec<u8> {
    if storage.len() < 16 {
        panic!("storage too short to contain Poly1305 tag");
    }
    
    let mut nonce_padded = [0u8; 12];
    nonce_padded[4..].copy_from_slice(nonce);
    
    let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(key));
    
    let result = cipher
    .decrypt(GenericArray::from_slice(&nonce_padded), Payload {
        msg: storage, // pass the entire ciphertext + tag buffer
        aad: aad.unwrap_or(&[]),
    });
    match result {
        Ok(value) => {return value},
        Err(error) => {log::error!("Error while decrypting with chacha20 poly1305: {}, returning vec![0]", error); return vec![0]}, // Print to standard error
    }
}

pub fn chacha20_poly1305_decrypt_all_64x64_nopad(
    key: &[u8; 32],
    nonce: &[u8; 8],
    storage: &[u8], // ciphertext + tag, as received
    aad: Option<&[u8]>,
) -> Vec<u8> {
    if storage.len() < 16 {
        panic!("storage too short to contain Poly1305 tag");
    }
    
    let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(key));
    
    let result = cipher
    .decrypt(GenericArray::from_slice(nonce.as_slice()), Payload {
        msg: storage, // pass the entire ciphertext + tag buffer
        aad: aad.unwrap_or(&[]),
    });
    match result {
        Ok(value) => {return value},
        Err(error) => {log::error!("Error while decrypting with chacha20 poly1305: {}, returning vec![0]", error); return vec![0]}, // Print to standard error
    }
}

pub fn handle_pair_setup(request_data: HttpRequest, pair_session: &mut Session) -> HttpResponse {
    let data: Vec<TlvField> = tlv_unmarshal(request_data.body);
    let mut stage: u8 = kPairSetupStateNone;
    let mut body: Vec<u8> = Vec::new();
    
    // find the state, honestly this should be in index 0, but i don't know if
    // it can change in the future so i'm going to let it loop thru
    for field in &data {
        if field.field_type == kTLVType_State {
            stage = field.data[0];
        }
    }
    let mut end: bool = false;
    match stage {
        kPairSetupStateM1 => body = handle_m1(pair_session),
        kPairSetupStateM3 => body = handle_m3(data, pair_session),
        kPairSetupStateM5 => {body = handle_m5(data, pair_session)},
        _ => {if stage != kPairSetupStateNone {
            log::error!("While handling the pair-setup endpoint an invalid (otherwise unhandled) PairSetupState was found, that being M{}", stage)
        } else {
            log::warn!("A request was received, however the PairSetupState was not set ({}), the body is most likely invalid.", stage);
        }},
    }
    return HttpResponse {
        status_code: String::from("200 OK"),
        protocol: String::from("RSTP/1.0"),
        headers: vec![],
        body: body,
        content_type: String::from("application/pairing+tlv8"),
        end,
    };
}

// Generating an M2 response, most likely to respond to an M1
pub fn handle_m1(pair_session: &mut Session) -> Vec<u8> {
    let PAIR_SETUP_USERNAME_RAW = CString::new("Pair-Setup").unwrap();
    let PAIR_SETUP_USERNAME = PAIR_SETUP_USERNAME_RAW.as_ptr() as *const c_void;
    let PAIR_SETUP_USERNAME_LEN = PAIR_SETUP_USERNAME_RAW.as_bytes().len();
    
    let PAIR_SETUP_PIN_RAW: CString = CString::new("3939").unwrap();
    let PAIR_SETUP_PIN = PAIR_SETUP_PIN_RAW.as_ptr() as *const c_void;
    let PAIR_SETUP_PIN_LEN = PAIR_SETUP_PIN_RAW.as_bytes().len();
    log::info!("↙️  Pair-Setup Server / M1 -- Processing M1 request");
    // fill random data
    let mut csprng = OsRng {};
    csprng.fill_bytes(&mut pair_session.salt);
    csprng.fill_bytes(&mut pair_session.b);
    
    // might be wrapped sooner or later, for now, it will be kept here
    let mut server_pk_ptr: *mut u8 = ptr::null_mut();
    let mut server_pk_len: size_t = 0;
    unsafe {
        let mut status = SRPCreate_libsrp(&mut pair_session.srp_ctx); // pass pointer-to-pointer
        if status == 0 {
            log::debug!("SRP object created at {:?}", pair_session.srp_ctx);
        } else {
            log::error!("Failed to create SRP object: {}", status);
        }
        
        status = SRPServerStart_libsrp(pair_session.srp_ctx,
            kSRPParameters_3072_SHA512,
            PAIR_SETUP_USERNAME,
            PAIR_SETUP_USERNAME_LEN,
            PAIR_SETUP_PIN,
            PAIR_SETUP_PIN_LEN,
            pair_session.salt.len(), 
            pair_session.salt.as_mut_ptr(),
            &mut server_pk_ptr,
            &mut server_pk_len
        );
        if status == 0 {
            println!("SRP Server started with parameters:
            Username: {:?}
            PIN: {:?}", PAIR_SETUP_USERNAME_RAW, PAIR_SETUP_PIN_RAW);
            pair_session.b_pub = Vec::from_raw_parts(server_pk_ptr, server_pk_len, server_pk_len);
        } else {
            log::error!("Failed to start SRP Server: {}", status);
            panic!("[FFI] Fatal error while starting the SRP server, please fix this, as the session cannot go on with this discrepancy. Error from function: {}", status);
        }
    }
    let fields = vec![
    // State = M2
    TlvField { field_type: 0x06, data: vec![0x02] },
    
    // SRP ephemeral server public key B
    TlvField { field_type: 0x03, data: pair_session.b_pub.clone() },
    
    // SRP salt
    TlvField { field_type: 0x02, data: pair_session.salt.to_vec() },
    ];
    log::info!("↗️  Pair-Setup Server / M2 -- Sending M2 response");
    return tlv_marshal(&fields);
}

// Generating an M4 response, most likely to respond to an M3
pub fn handle_m3(data: Vec<TlvField>, pair_session: &mut Session) -> Vec<u8> {
    log::info!("↙️  Pair-Setup Server / M3 -- Processing M3 request"); 
    
    let mut a_pub_buf: Vec<u8> = Vec::new();
    let mut proof_buf: Vec<u8> = Vec::new();
    for mut field in data {
        if field.field_type == kTLVType_PublicKey {
            a_pub_buf.extend_from_slice(&mut field.data);
        }else if field.field_type == kTLVType_Proof {
            proof_buf.extend_from_slice(&mut field.data);
        }
    }
    let mut shared_secret: *mut u8 = ptr::null_mut();
    let mut shared_secret_len: size_t = 0;
    let mut response: *mut u8 = ptr::null_mut();
    let mut response_len: size_t = 0;
    let mut cured_response: Vec<u8>;
    
    // SRP Server handling, will be wrapped soon for easier readability
    unsafe {
        let mut srp_ctx: *mut SRPPrivate_libsrp = std::ptr::null_mut();
        let mut status = SRPServerVerify_libsrp(pair_session.srp_ctx,
            a_pub_buf.as_slice().as_ptr() as *const c_void,
            a_pub_buf.as_slice().len(),
            proof_buf.as_slice().as_ptr() as *const c_void,
            proof_buf.as_slice().len(),
            &mut shared_secret,
            &mut shared_secret_len,
            &mut response,
            &mut response_len
        );
        if status == 0 {
            log::debug!("SRP Server has successfully verified the client");
            cured_response = unsafe {
                Vec::from_raw_parts(response, response_len, response_len)
            };
            pair_session.shared_secret = unsafe {
                Vec::from_raw_parts(shared_secret, shared_secret_len, shared_secret_len)
            };
        } else {
            log::error!("Failed to verify the client response: {}", status);
            panic!("[FFI] [pair-setup.rs] Fatal error while verifying client response, please fix this, as the session cannot go on with this discrepancy. Error from function: {}", status);
        }
    }
    
    // HKDF logic, store the normally derived key to ps_encrypt_key
    Hkdf::<Sha512>::new(Some(&kPairSetupEncryptSaltPtr), &pair_session.shared_secret)
    .expand(kPairSetupEncryptInfoPtr, &mut pair_session.encrypt_key)
    .expect("Error found in HKDF process");
    
    // HKDF logic, store the MFi derived key to ps_mfi_encrypt_key
    let mut msg: [u8; 32] = [0; 32];
    Hkdf::<Sha512>::new(Some(&kMFiPairSetupSaltPtr), &pair_session.shared_secret)
    .expand(kMFiPairSetupInfoPtr, &mut pair_session.mfi_encrypt_key)
    .expect("Error found in HKDF process");
    
    let mut hasher = Sha1::new();
    hasher.update(msg);
    let signature = cp2::rsa_sign(&pair_session.i2c_device, hasher.finalize().as_slice()).expect("Error signing with MFi");
    let mut i2cd;
    if pair_session.i2c_device.is_some() {
        i2cd = &pair_session.i2c_device;
    }
    let encryptedfields = vec![
    // Signature
    TlvField { field_type: kTLVType_Signature, data: signature },
    
    // MFi IC Certificate
    TlvField { field_type: kTLVType_Certificate, data: cp2::ret_cp_pubkey(i2cd) },
    ];
    let nonce = b"PS-Msg04"; // exactly 8 bytes
    let (mut ciphertext, tag) = chacha20_poly1305_encrypt_all_64x64(&pair_session.encrypt_key, &nonce, tlv_marshal(&encryptedfields).as_slice(), None);
    ciphertext.extend_from_slice(&tag);
    // Encode everything to readable TLV and marshal it to send it off
    let fields = vec![
    // State = M4
    TlvField { field_type: 0x06, data: vec![kPairSetupStateM4] },
    
    // SRP ephemeral server public key B
    TlvField { field_type: kTLVType_Proof, data: cured_response },
    
    // Encrypted data
    TlvField { field_type: kTLVType_EncryptedData, data: ciphertext }
    ];
    log::info!("↗️  Pair-Setup Server / M4 -- Sending M4 response");
    return tlv_marshal(&fields);
}

fn handle_m5(data: Vec<TlvField>, pair_session: &mut Session) -> Vec<u8> {
    // redefine data as a clone of the original data, but now made mut
    let mut data = data.clone();
    log::info!("↙️  Pair-Setup Server / M5 -- Processing M5 request");
    let mut encrypted_buf: Vec<u8> = Vec::new();
    for field in data.iter() {
        if field.field_type == kTLVType_EncryptedData {
            encrypted_buf.extend_from_slice(&field.data);
        }
    }
    let nonce = b"PS-Msg05"; // exactly 8 bytes
    // replace TLV body with the decrypted version
    data = tlv_unmarshal(chacha20_poly1305_decrypt_all_64x64(&pair_session.encrypt_key, &nonce, &encrypted_buf, None));
    
    // parse new TLV to buffers and session data
    let mut peer_sig_buf: Vec<u8> = Vec::new();
    for field in data.iter() {
        if field.field_type == kTLVType_Identifier {
            pair_session.peer_identifier.extend_from_slice(&field.data);
        } else if field.field_type == kTLVType_PublicKey {
            pair_session.peer_ed_pk.extend_from_slice(&field.data);
        } else if field.field_type == kTLVType_Signature {
            peer_sig_buf.extend_from_slice(&field.data);
        }
    }
    let mut controller_key_buf: Vec<u8> = Vec::with_capacity(32 + pair_session.peer_identifier.len() + 32);
    let mut hk_out = [0u8; 32];
    
    // HKDF logic and concatenade all data to feed into Verifier
    Hkdf::<Sha512>::new(Some(&kPairSetupControllerSignSalt), &pair_session.shared_secret)
    .expand(kPairSetupControllerSignInfo, &mut hk_out)
    .expect("Error found in HKDF process");
    controller_key_buf.extend_from_slice(&hk_out);
    controller_key_buf.extend_from_slice(&pair_session.peer_identifier);
    controller_key_buf.extend_from_slice(&pair_session.peer_ed_pk);
    
    // Prepare data for feeding into Verifier
    let mut pepk: [u8; 32] = [0; 32];
    pepk.copy_from_slice(&pair_session.peer_ed_pk);
    let mut ps: [u8; 64] = [0; 64];
    ps.copy_from_slice(&peer_sig_buf);
    
    // Ed25519 logic
    let ver = VerifyingKey::from_bytes(&pepk).expect("Error while getting VerifyingKey from_bytes");
    let sig = Signature::from_bytes(&ps);
    match ver.verify(&controller_key_buf, &sig) {
        Ok(_) => log::debug!("Successfully verified"),
        Err(_) => {
            log::error!("Failed to verify controller_key_buf");
            let fields = vec![
            TlvField { field_type: kTLVType_State, data: vec![kPairSetupStateM6] },
            TlvField { field_type: kTLVType_Error, data: vec![kTLVError_Authentication] },
            ];
            return tlv_marshal(&fields);
        }
    }
    persister::write_peer_data(pair_session.peer_identifier.clone(), pair_session.peer_ed_pk.clone());

    // Start processing response
    // Fetch all the falues from the persister
    let mut pairing_identity: Vec<u8>;
    match persister::read_key(CFG_PAIRING_IDENTITY) {
        Ok(pid) => pairing_identity = pid,
        Err(e) => panic!("Fatal! Error while trying to fetch the Pairing Identity, \"{}\". While the program can go on, it is a good practice to fix this.", e),
    }
    match persister::read_key(CFG_ED25519_PK) {
        Ok(buf) => pair_session.host_ed_pk = buf,
        Err(e) => panic!("Fatal! Error while trying to fetch the Ed25519 PK: {}", e)
    }
    match persister::read_key(CFG_ED25519_SK) {
        Ok(buf) => pair_session.host_ed_sk = buf,
        Err(e) => panic!("Fatal! Error while trying to fetch the Ed25519 SK: {}", e)
    }
    
    // HKDF logic
    let mut accessory_key_buf: Vec<u8> = Vec::with_capacity(32 + pairing_identity.len() + 32);
    
    Hkdf::<Sha512>::new(Some(&kPairSetupAccessorySignSalt), &pair_session.shared_secret)
    .expand(kPairSetupAccessorySignInfo, &mut hk_out)
    .expect("Error found in HKDF process");
    
    // Start appending everything
    accessory_key_buf.extend_from_slice(&hk_out);
    accessory_key_buf.extend_from_slice(&pairing_identity);
    accessory_key_buf.extend_from_slice(&pair_session.host_ed_pk);
    
    let raw_sk: [u8; 32] = pair_session.host_ed_sk.clone().try_into().expect("Expected a 32-byte Vec");
    let sk = SigningKey::from_bytes(&raw_sk);
    let data_signature: Signature = sk.sign(&accessory_key_buf);
    
    // Encrypted fields
    let encryptedfields = vec![
    TlvField { field_type: kTLVType_Identifier, data: pairing_identity.clone() },
    TlvField { field_type: kTLVType_PublicKey, data: pair_session.host_ed_pk.clone() },
    TlvField { field_type: kTLVType_Signature, data: data_signature.to_vec() },
    ];
    let nonce = b"PS-Msg06"; // exactly 8 bytes
    let (mut ciphertext, tag) = chacha20_poly1305_encrypt_all_64x64(&pair_session.encrypt_key, &nonce, tlv_marshal(&encryptedfields).as_slice(), None);
    ciphertext.extend_from_slice(&tag);
    
    let fields = vec![    
    // Encrypted data
    TlvField { field_type: kTLVType_EncryptedData, data: ciphertext },
    
    // State = M6
    TlvField { field_type: 0x06, data: vec![kPairSetupStateM6] },
    ];
    log::info!("↗️  Pair-Setup Server / M6 -- Sending M6 response");
    log::info!("✅ Pair-Setup process completed!");
    return tlv_marshal(&fields);
}