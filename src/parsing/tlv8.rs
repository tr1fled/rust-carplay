// TLv8 parsing tools
// added here for shorter files and (i guess) convenience

// thanks tim apple for this
pub const kTLVType_Identifier: u8 =				0x01; // Identifier of the peer.
pub const kTLVType_Salt: u8 =					0x02; // 16+ bytes of random salt.
pub const kTLVType_PublicKey: u8 =				0x03; // Curve25519, SRP public key, or signed Ed25519 key.
pub const kTLVType_Proof: u8 =					0x04; // SRP proof.
pub const kTLVType_EncryptedData: u8 =			0x05; // Encrypted bytes. Use AuthTag to authenticate.
pub const kTLVType_State: u8 =					0x06; // State of the pairing process.
pub const kTLVType_Error: u8 =					0x07; // Error code. Missing means no error.
pub const kTLVType_RetryDelay: u8 =				0x08; // Seconds to delay until retrying setup.
pub const kTLVType_Certificate: u8 =			0x09; // X.509 Certificate.
pub const kTLVType_Signature: u8 =				0x0A; // Ed25519 or MFi auth IC signature.
pub const kTLVType_ReservedB: u8 =				0x0B; // Reserved.
pub const kTLVType_FragmentData: u8 =			0x0C; // Non-last fragment of data. If length is 0, it's an ack.
pub const kTLVType_FragmentLast: u8 =			0x0D; // Last fragment of data.

pub const kTLVError_Reserved0: u8 =				0x00; // Must not be used in any TLV.
pub const kTLVError_Unknown: u8 =				0x01; // Generic error to handle unexpected errors.
pub const kTLVError_Authentication: u8 =		0x02; // Setup code or signature verification failed.
pub const kTLVError_Backoff: u8 =				0x03; // Client must look at <RetryDelay> TLV item and wait before retrying.
pub const kTLVError_UnknownPeer: u8 =			0x04; // Peer is not paired.
pub const kTLVError_MaxPeer: u8 =				0x05; // Server cannot accept any more pairings.
pub const kTLVError_MaxTries: u8 =				0x06; // Server reached its maximum number of authentication attempts

#[derive(Debug, Clone)]
pub struct TlvField {
    pub field_type: u8,
    pub data: Vec<u8>,
}

pub fn tlv_marshal(fields: &[TlvField]) -> Vec<u8> {
    let mut result = Vec::new();
    
    for m in fields {
        let mut remaining = &m.data[..];
        while !remaining.is_empty() {
            let chunk_len = remaining.len().min(255);
            result.push(m.field_type);
            result.push(chunk_len as u8);
            result.extend_from_slice(&remaining[..chunk_len]);
            remaining = &remaining[chunk_len..];
        }
    }
    
    result
}

pub fn tlv_unmarshal(buf: Vec<u8>) -> Vec<TlvField> {
    let mut ret = Vec::new();
    let mut index = 0;

    while index + 2 <= buf.len() {
        let field_type = buf[index];
        let length = buf[index + 1] as usize;
        let start = index + 2;
        let end = start + length;
        if field_type == 0 {index = end; continue};
        // If declared length goes past buffer, stop parsing
        if end > buf.len() {
            break;
        }
        let data = buf[start..end].to_vec();
        ret.push(TlvField {
            field_type,
            data,
        });
        index = end;
    }
    ret
}