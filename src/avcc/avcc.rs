/// Convert an AVCConfiguration record to an AnnexB formatted one
/// 
/// # Arguments
/// * `in_avcc` - `Vec<u8>` containing the AVCConfiguration record, given to the screen receiver as opcode 1.
/// 
/// # Returns
/// * `out_annexb` - Vec<u8> containing the configuration record converted to AnnexB format.
pub fn avcc_to_annexb(in_avcc: Vec<u8>) -> Vec<u8> {
    // AVCC Format is documented in ISO/IEC STANDARD 14496-15 (AVC file format) section 5.2.4.1.1.
    //
    // [0x00] version = 1.
    // [0x01] AVCProfileIndication		Profile code as defined in ISO/IEC 14496-10.
    // [0x02] Profile Compatibility		Byte between profile_IDC and level_IDC in SPS from ISO/IEC 14496-10.
    // [0x03] AVCLevelIndication		Level code as defined in ISO/IEC 14496-10.
    // [0x04] LengthSizeMinusOne		0b111111xx where xx is 0, 1, or 3 mapping to 1, 2, or 4 bytes for nal_size.
    // [0x05] SPSCount					0b111xxxxx where xxxxx is the number of SPS entries that follow this field.
    // [0x06] SPS entries				Variable-length SPS array. Each entry has the following structure:
    //		uint16_t	spsLen			Number of bytes in the SPS data until the next entry or the end (big endian).
    //		uint8_t		spsData			SPS entry data.
    // [0xnn] uint8_t	PPSCount		Number of Picture Parameter Sets (PPS) that follow this field.
    // [0xnn] PPS entries				Variable-length PPS array. Each entry has the following structure:
    //		uint16_t	ppsLen			Number of bytes in the PPS data until the next entry or the end (big endian).
    //		uint8_t		ppsData			PPS entry data.
    //
    // Annex-B format is documented in the H.264 spec in Annex-B.
    // Each NAL unit is prefixed with 0x00 0x00 0x00 0x01 and the nal_size from the AVCC is removed.
    let nalsize = ( in_avcc[4] & 0x03 ) + 1;
    let mut iter = in_avcc[ 5 ] & 0x1F;
    let mut src = &in_avcc[6..];
    let mut src_read_index: usize = 0;
    let mut ec = 0;
    let mut len: usize;
    let mut dst: Vec<u8> = Vec::new();
    
    // Q -> why do we use read_index?
    // A -> because in the C code, we see things like
    // src +=2, which means add 2 to the reading index,
    // ex: src[0], if we do src +=2 then it becomes src[2]
    // and reads/writes will start from 2 even when you do
    // src[0], because you have selected to start operations at
    // that index.
    while ec < iter {
        len = (((src[src_read_index] as usize) << 8) | src[src_read_index + 1] as usize) as usize;
        // advance 2 bytes
        src_read_index += 2;
        
        // annexb header i think
        dst.push(0x00);
        dst.push(0x00);
        dst.push(0x00);
        dst.push(0x01);
        dst.extend_from_slice(&src[src_read_index..src_read_index + len]);
        
        src_read_index += len;
        // increment counter
        ec += 1;
    };
    ec = 0;
    if src_read_index + 1 < src.len() {
        iter = src[src_read_index];
        src_read_index += 1;
        while ec < iter {
            len = (((src[src_read_index] as usize) << 8) | src[src_read_index + 1] as usize) as usize;
            // advance 2 bytes
            src_read_index += 2;
            
            // annexb header i think
            dst.push(0x00);
            dst.push(0x00);
            dst.push(0x00);
            dst.push(0x01);
            dst.extend_from_slice(&src[src_read_index..src_read_index + len]);
            
            src_read_index += len;
            // increment counter
            ec += 1;
        }
    }
    dst
}

/// Convert a length prefixed NAL unit to one with an AnnexB start code
/// 
/// # Arguments
/// * `in_avc` - `Vec<u8>` containing the length-prefixed NALu
/// 
/// # Returns
/// * `out_annexb` - `Vec<u8>` with an AnnexB start code (00, 00, 00, 01)
pub fn avc_to_annexb(in_avc: &[u8]) -> Vec<u8> {
let mut out_annexb = Vec::with_capacity(in_avc.len() + 32);
    let mut i = 0;
    let mut data = in_avc;

    while i + 4 <= data.len() {
        // Read 4-byte length (big-endian)
        let nalu_len = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        i += 4;

        // Bounds check
        if i + nalu_len > data.len() {
            break;
        }

        // Write Annex B start code
        out_annexb.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);

        // Write NALU payload
        out_annexb.extend_from_slice(&data[i..i + nalu_len]);

        // Advance
        i += nalu_len;
    }

    out_annexb
}