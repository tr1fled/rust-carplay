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
use log;
use rand::{rngs::OsRng, RngCore};
use sha2::{Sha256, Sha512};
use sha1::{Digest, Sha1};
use hkdf::Hkdf;
use x25519_dalek::{PublicKey, StaticSecret};
use chacha20poly1305::{
    aead::{Aead, KeyInit, generic_array::GenericArray},
    ChaCha20Poly1305, // 96-bit nonce version (standard)
    XChaCha20Poly1305 // 192-bit nonce version
};
use chacha20poly1305::aead::Payload;
use crate::parsing::tlv8::*;
use crate::parsing::http::{
    HttpRequest,
    HttpResponse
};
use crate::ssl::bindings::{self, SHA1_Final, SHA1_Init, SHA1_Update, SHA_CTX};
use crate::ctr::bindings::{AES_CTR_Init, AES_CTR_Update, AES_CTR_Final, AES_CTR_Context};
use crate::networking::http_responders::pair_setup::Session;
// change as you wish, since we do not have an mfi chip we will 'troll' apple
use crate::mfi::cp2::{self, ret_cp_pubkey};
use crate::persister::{self, CFG_ED25519_PK, CFG_ED25519_SK, CFG_PAIRING_IDENTITY};
use std::ffi::{CStr, CString};


// thanks tim apple for the spacings and all

const kAuthSetupStateNone: u8 =         0; // Controller -> Accessory  -- Start Request
const kAuthSetupStateM1: u8 =           1; // Controller -> Accessory  -- Start Request
const kAuthSetupStateDone: u8 =         7;

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

fn chacha20_poly1305_decrypt_all_64x64(
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

pub fn handle_auth_setup(request_data: HttpRequest, pair_session: &mut Session) -> HttpResponse {
    let mut end: bool = false;
    let mut body: Vec<u8> = Vec::new();
    
    body = handle_m1(request_data.body, pair_session);
    
    return HttpResponse {
        status_code: String::from("200 OK"),
        protocol: String::from("RSTP/1.0"),
        headers: vec![],
        body: body,
        content_type: String::from("application/pairing+tlv8"),
        end,
    };
}

fn handle_m1(data: Vec<u8>, pair_session: &mut Session) -> Vec<u8> {
    log::info!("↙️  Auth-Setup Server / Processing request");
    let mut peer_ec_pk: [u8; 32] = [0; 32];
    let mut host_ec_sk: [u8; 32] = [0; 32];
    let mut shared_secret_vec = Vec::new();
    let mut sha1_ctx = std::mem::MaybeUninit::<SHA_CTX>::uninit();
    let mut aes_ctx = std::mem::MaybeUninit::<AES_CTR_Context>::uninit();
    let AES_KEY_RAW = CString::new("AES-KEY").unwrap();
    let AES_KEY = AES_KEY_RAW.as_ptr() as *const c_void;
    let AES_IV_RAW = CString::new("AES-IV").unwrap();
    let AES_IV = AES_IV_RAW.as_ptr() as *const c_void;
    
    // get peer Ed PK
    peer_ec_pk.copy_from_slice(&data[1..]);
    
    let mut csprng = OsRng {};
    csprng.fill_bytes(&mut host_ec_sk);
    assert!(pair_session.peer_ed_pk.len() == 32);
    
    let host_secret = StaticSecret::from(host_ec_sk);
    let host_public = PublicKey::from(&host_secret);
    let mut peer_public = PublicKey::from(peer_ec_pk);
    let shared_secret = host_secret.diffie_hellman(&peer_public); // as SharedSecret
    shared_secret_vec = shared_secret.to_bytes().to_vec(); // as Vec
    
    // get AES KEY
    let mut aes_key = [0u8; 20];
    unsafe {
        println!("Beginning SHA1 AES KEY process");
        SHA1_Init(sha1_ctx.as_mut_ptr());
        println!("init");
        SHA1_Update(sha1_ctx.as_mut_ptr(), AES_KEY, AES_KEY_RAW.as_bytes().len());
        println!("up1");
        SHA1_Update(sha1_ctx.as_mut_ptr(), shared_secret.to_bytes().as_ptr() as *const c_void, shared_secret.to_bytes().len());
        println!("up2");
        SHA1_Final(aes_key.as_mut_ptr(), sha1_ctx.as_mut_ptr());
        println!("fin");
    }
    let mut aes_key_ptr = aes_key.as_mut_ptr();
    
    // get AES IV
    let mut aes_iv = [0u8; 20];
    unsafe {
        println!("Beginning SHA1 AES IV process");
        SHA1_Init(sha1_ctx.as_mut_ptr());
        SHA1_Update(sha1_ctx.as_mut_ptr(), AES_IV, AES_IV_RAW.as_bytes().len());
        SHA1_Update(sha1_ctx.as_mut_ptr(), shared_secret.to_bytes().as_ptr() as *const c_void, shared_secret.to_bytes().len());
        SHA1_Final(aes_iv.as_mut_ptr(), sha1_ctx.as_mut_ptr());
    }

    let mut aes_iv_ptr = aes_iv.as_mut_ptr();

    // our pk + peer pk
    let mut hasher = Sha256::new();
    hasher.update(&host_public.to_bytes());
    hasher.update(&peer_ec_pk);

    // sign with mfi
    let cert = ret_cp_pubkey(pair_session.i2c_device.clone());
    let mut signature = cp2::rsa_sign(pair_session.i2c_device.clone(), &hasher.finalize()).expect("Error signing with MFi");
    println!("debug cert (auth-setup):\n\ncert: {:#?}\ncert_len: {:#?}", cert, cert.len());
    let mut signature_out = vec![0u8; signature.len()];
    unsafe {
        println!("Beginning AES process");
        let mut err = AES_CTR_Init(aes_ctx.as_mut_ptr(), aes_key_ptr as *const u8, aes_iv_ptr as *const u8);
        println!("Error value of INIT: {}", err);
        AES_CTR_Update(aes_ctx.as_mut_ptr(), signature.as_ptr() as *const c_void, signature.len(), signature_out.as_mut_ptr() as *mut c_void);
        println!("Error value of Update: {}", err);
        AES_CTR_Final(aes_ctx.as_mut_ptr());
        println!("Error value of Final: {}", err);
    }
    //let mut cipher = Aes128Ctr64BE::new(aes_key[..16].into(), aes_iv[..16].into());
    //cipher.apply_keystream(&mut signature);
    // signature has been rewritten. moving on
    // Return the response:
    //
    //		<32:our ECDH public key>
    //		<4:big endian certificate length>
    //		<N:certificate data>
    //		<4:big endian signature length>
    //		<N:encrypted signature data>
    let cert_len_be = (cert.len() as u32).to_be_bytes();       // 4 bytes
    let sig_len_be  = (signature_out.len() as u32).to_be_bytes(); // 4 bytes
    
    let mut buf: Vec<u8> = Vec::with_capacity(
        32 + 4 + cert.len() + 4 + signature_out.len()
    );
    
    buf.extend_from_slice(&host_public.to_bytes());   // 32 bytes
    buf.extend_from_slice(&cert_len_be);             // 4 bytes
    buf.extend_from_slice(&cert);                    // cert bytes
    buf.extend_from_slice(&sig_len_be);              // 4 bytes
    buf.extend_from_slice(&signature_out);           // sig bytes
    
    assert_eq!(buf.len(), 32 + 4 + cert.len() + 4 + signature_out.len());
    log::info!("↗️  Auth-Setup Server / Sending response");
    return buf;
}