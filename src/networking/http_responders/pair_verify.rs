// thanks to the beautiful people at dalek cryptography
// who have made beautiful crypto libraries

#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[allow(dead_code)]
use log;
use rand::{rngs::OsRng, RngCore};
use sha2::{Sha512};
use hkdf::Hkdf;
use chacha20poly1305::{
    aead::{Aead, KeyInit, generic_array::GenericArray},
    ChaCha20Poly1305, // 96-bit nonce version (standard)
};
use chacha20poly1305::aead::Payload;
use crate::parsing::tlv8::*;
use crate::parsing::http::{
    HttpRequest,
    HttpResponse
};
use x25519_dalek::{PublicKey, StaticSecret};
use ed25519_dalek::*;
// change as you wish, since we do not have an mfi chip we will 'troll' apple
//use crate::mfi::fake_mfi::*;
use crate::mfi::cp2::*;
use crate::persister::{self, CFG_ED25519_PK, CFG_ED25519_SK, CFG_PAIRING_IDENTITY};
use crate::networking::http_responders::pair_setup::Session;


// thanks tim apple for the spacings and all

// ECDH
const kPairVerifyECDHSalt: &[u8] =				b"Pair-Verify-ECDH-Salt";
const kPairVerifyECDHInfo: &[u8] =				b"Pair-Verify-ECDH-Info";
const kPairVerifyEncryptSalt: &[u8] =			b"Pair-Verify-Encrypt-Salt";
const kPairVerifyEncryptInfo: &[u8] =			b"Pair-Verify-Encrypt-Info";

const kPairVerifyStateNone: u8 =            0; // Controller -> Accessory  -- Start Request
const kPairVerifyStateM1: u8 =              1; // Controller -> Accessory  -- Start Request
const kPairVerifyStateM2: u8 =              2; // Accessory  -> Controller -- Start Response
const kPairVerifyStateM3: u8	=           3; // Controller -> Accessory  -- Verify Request
const kPairVerifyStateM4: u8	=           4; // Accessory  -> Controller -- Verify Response
const kPairVerifyStateDone: u8 =            7;

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


pub fn handle_pair_verify(request_data: HttpRequest, pair_session: &mut Session) -> HttpResponse {
    let data: Vec<TlvField> = tlv_unmarshal(request_data.body);
    let mut stage: u8 = kPairVerifyStateNone;
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
        kPairVerifyStateM1 => body = handle_m1(data, pair_session),
        kPairVerifyStateM3 => {body = handle_m3(data, pair_session); end=true},
        _ => {if stage != kPairVerifyStateNone {
            log::error!("While handling the pair-verify endpoint an invalid (otherwise unhandled) PairVerifyState was found, that being M{}", stage)
        } else {
            log::warn!("A request was received, however the PairVerifyState was not set ({}), the body is most likely invalid.", stage)
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

fn handle_m1(data: Vec<TlvField>, pair_session: &mut Session) -> Vec<u8> {
    log::info!("↙️  Pair-Verify Server / M1 -- Processing M1 request");
    // Curve25519 & HKDF logic, Curve keypair generation compliant with the
    // original spec, but the reasons to do this might be little to none?
    let mut hkdf_buf = [0u8; 32];
    let mut csprng = OsRng {};
    csprng.fill_bytes(&mut hkdf_buf);
    Hkdf::<Sha512>::new(Some(&kPairVerifyECDHSalt), &hkdf_buf)
    .expand(kPairVerifyECDHInfo, &mut hkdf_buf)
    .expect("Error found in HKDF process");
    let host_secret = StaticSecret::from(hkdf_buf);
    let host_public = PublicKey::from(&host_secret);
    let mut peer_public = PublicKey::from([0; 32]); // stupid ugly hack to make the compiler/analyzer shut up
    pair_session.host_curve_sk.extend_from_slice(&host_secret.to_bytes());
    pair_session.host_curve_pk.extend_from_slice(&host_public.to_bytes());
    
    // fill session data
    for mut field in data {
        if field.field_type == kTLVType_PublicKey {
            pair_session.peer_curve_pk = field.data.clone(); // ✅ critical line
            peer_public = PublicKey::from(<[u8; 32]>::try_from(field.data).expect("Expected 32-byte public key"));
        }
    }
    
    let shared_secret = host_secret.diffie_hellman(&peer_public); // as SharedSecret
    pair_session.shared_secret = shared_secret.to_bytes().to_vec(); // as Vec
    
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
    
    let raw_sk: [u8; 32] = pair_session.host_ed_sk.clone().try_into().expect("Expected a 32-byte Vec");
    let sk = SigningKey::from_bytes(&raw_sk);
    let mut accessory_key_buf: Vec<u8> = Vec::with_capacity(32 + pairing_identity.len() + 32);
    accessory_key_buf.extend_from_slice(&pair_session.host_curve_pk);
    accessory_key_buf.extend_from_slice(&pairing_identity);
    accessory_key_buf.extend_from_slice(&pair_session.peer_curve_pk);
    let data_signature: Signature = sk.sign(&accessory_key_buf);
    
    Hkdf::<Sha512>::new(Some(&kPairVerifyEncryptSalt), &pair_session.shared_secret)
    .expand(kPairVerifyEncryptInfo, &mut pair_session.encrypt_key)
    .expect("Error found in HKDF process");
    
    let encryptedfields = vec![
        // Pairing Identity
        TlvField { field_type: kTLVType_Identifier, data: pairing_identity },
    
        // Data signature
        TlvField { field_type: kTLVType_Signature, data: data_signature.to_vec() },
    ];
    let (mut ciphertext, tag) = chacha20_poly1305_encrypt_all_64x64(&pair_session.encrypt_key, b"PV-Msg02", tlv_marshal(&encryptedfields).as_slice(), None);
    ciphertext.extend_from_slice(&tag);
    
    let fields = vec![    
        // Encrypted Data
        TlvField { field_type: kTLVType_EncryptedData, data: ciphertext },

        // Public Key
        TlvField { field_type: kTLVType_PublicKey, data: pair_session.host_curve_pk.clone() },

        // State = M2
        TlvField { field_type: 0x06, data: vec![kPairVerifyStateM2] },
    ];
    log::info!("↗️  Pair-Verify Server / M2 -- Sending M2 response");
    return tlv_marshal(&fields);
}

fn handle_m3(data: Vec<TlvField>, pair_session: &mut Session) -> Vec<u8> {
    log::info!("↙️  Pair-Verify Server / M3 -- Processing M3 request");
    let mut data = data.clone();
    let mut encrypted_buf: Vec<u8> = Vec::new();
    let mut peer_sig_buf: Vec<u8> = Vec::new();
    // fill session data
    for mut field in data {
        if field.field_type == kTLVType_EncryptedData {
            encrypted_buf.extend_from_slice(&field.data)
        }
    }
    // Read peer Ed PK from the persister
    data = tlv_unmarshal(chacha20_poly1305_decrypt_all_64x64(&pair_session.encrypt_key, b"PV-Msg03", &encrypted_buf, None));
    for mut field in data {
        if field.field_type == kTLVType_Identifier {
            pair_session.peer_identifier.clear();
            pair_session.peer_identifier.extend_from_slice(&field.data);
            pair_session.peer_ed_pk = persister::read_peer_data(field.data);
        } else if field.field_type == kTLVType_Signature {
            peer_sig_buf.extend_from_slice(&field.data);
        }
    }
    // Prepare data for feeding into Verifier
    let mut pepk: [u8; 32] = [0; 32];
    pepk.copy_from_slice(&pair_session.peer_ed_pk);
    let mut ps: [u8; 64] = [0; 64];
    ps.copy_from_slice(&peer_sig_buf);
    let mut controller_key_buf: Vec<u8> = Vec::with_capacity(32 + pair_session.peer_identifier.len() + 32);
    assert!(pair_session.peer_curve_pk.len() == 32);
    log::info!("THE LENGTH OF THE PEER ID IS AS FOLLOWS: {}", pair_session.peer_identifier.len());
    assert!(pair_session.peer_ed_pk.len() == 32);
    controller_key_buf.extend_from_slice(&pair_session.peer_curve_pk);
    controller_key_buf.extend_from_slice(&pair_session.peer_identifier);
    controller_key_buf.extend_from_slice(&pair_session.host_curve_pk);

    // Ed25519 logic
    let ver = VerifyingKey::from_bytes(&pepk).expect("Error while getting VerifyingKey from_bytes");
    let sig = Signature::from_bytes(&ps);
    match ver.verify(&controller_key_buf, &sig) {
        Ok(_) => log::debug!("Successfully verified"),
        Err(_) => {
            log::error!("Failed to verify controller_key_buf");
            let fields = vec![
            TlvField { field_type: kTLVType_State, data: vec![kPairVerifyStateM4] },
            TlvField { field_type: kTLVType_Error, data: vec![kTLVError_Authentication] },
            ];
            return tlv_marshal(&fields);
        }
    }
    pair_session.encrypted = true;
    let fields = vec![    
        // State = M4
        TlvField { field_type: 0x06, data: vec![kPairVerifyStateM4] },
    ];
    log::info!("↗️  Pair-Verify Server / M4 -- Sending M4 response");
    log::info!("✅ Pair-Verify process completed!");
    return tlv_marshal(&fields);
}