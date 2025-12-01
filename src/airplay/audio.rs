use serde::Serialize;
#[derive(Serialize)]
pub struct AudioFmtData {
    #[serde(skip_serializing_if = "is_zero")]
    audioInputFormats: u64,
    #[serde(skip_serializing_if = "is_zero")]
    audioOutputFormats: u64,
    audioType: String,
    r#type: u64,
}
#[derive(Serialize)]
pub struct AudioLatData {
    inputLatencyMicros: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    audioType: Option<String>,
    r#type: u64,
    //sr: u64, // Uncomment these three if needed,
    //ss: u64, // but if it's not required and it
    //ch: u64, // ain't broke, dont fix it.
    outputLatencyMicros: u64,
}
#[derive(Debug, Clone)]
pub struct AudioStreamBasicDescription { // ASBD
    pub sample_rate: u32,
    pub format_id: u32,
    pub format_flags: u32,
    pub bytes_per_packet: u32,
    pub frames_per_packet: u32,
    pub bytes_per_frame: u32,
    pub channels_per_frame: u32,
    pub bits_per_channel: u32,
    pub reserved: u32,
}
impl AudioStreamBasicDescription {
    /// Fills an AudioStreamBasicDescriptor (ASBD) with AAC-LC stream data
    /// 
    /// # Arguments
    /// `rate` - `u32` containing the audio sample rate
    /// `channels` - `u32` containing the audio channel amount
    /// 
    /// # Returns
    /// `AudioStreamBasicDescription` containing  the stream description
    pub fn fill_aac_lc(rate: u32, channels: u32) -> Self {
        Self {
            sample_rate: rate,
            format_id: kAudioFormatMPEG4AAC.try_into().expect("Failure should not happen, yet it did"),
            format_flags: 0,
            bytes_per_packet: 0,
            frames_per_packet: kAudioSamplesPerPacket_AAC_LC.try_into().expect("Failure should not happen, yet it did"),
            bytes_per_frame: 0,
            channels_per_frame: channels,
            bits_per_channel: 16,
            reserved: 0,
        }
    }
    /// Fills an AudioStreamBasicDescriptor (ASBD) with AAC-ELD stream data
    /// 
    /// # Arguments
    /// `rate` - `u32` containing the audio sample rate
    /// `channels` - `u32` containing the audio channel amount
    /// 
    /// # Returns
    /// `AudioStreamBasicDescription` containing the stream description
    pub fn fill_aac_eld(rate: u32, channels: u32) -> Self {
        Self {
            sample_rate: rate,
            format_id: kAudioFormatMPEG4AAC_ELD.try_into().expect("Failure should not happen, yet it did"),
            format_flags: 0,
            bytes_per_packet: 0,
            frames_per_packet: kAudioSamplesPerPacket_AAC_ELD.try_into().expect("Failure should not happen, yet it did"),
            bytes_per_frame: 0,
            channels_per_frame: channels,
            bits_per_channel: 16,
            reserved: 0,
        }
    }
    /// Fills an AudioStreamBasicDescriptor (ASBD) with PCM stream data
    /// 
    /// # Arguments
    /// `rate` - `u32` containing the audio sample rate
    /// `valid_bits` - `u32`
    /// `total_bits` - `u32`
    /// `channels` - `u32` containing the audio channel amount
    /// 
    /// # Returns
    /// `AudioStreamBasicDescription` containing the stream description
    pub fn fill_pcm(rate: u32, valid_bits: u32, total_bits: u32, channels: u32) -> Self {
        Self {
            sample_rate: rate,
            format_id: kAudioFormatLinearPCM.try_into().expect("Failure should not happen, yet it did"),
            format_flags: (kAudioFormatFlagsNativeEndian | kAudioFormatFlagIsSignedInteger | if valid_bits == total_bits {kAudioFormatFlagIsPacked} else {kAudioFormatFlagIsAlignedHigh}).try_into().expect("Failure should not happen, yet it did"),
            bytes_per_packet: channels * (total_bits / 8),
            frames_per_packet: 1,
            bytes_per_frame: channels * (total_bits / 8),
            channels_per_frame: channels,
            bits_per_channel: valid_bits,
            reserved: 0,
        }
    }
    /// Fills an AudioStreamBasicDescriptor (ASBD) with Opus stream data
    /// 
    /// # Arguments
    /// `rate` - `u32` containing the audio sample rate
    /// `channels` - `u32` containing the audio channel amount
    /// 
    /// # Returns
    /// `AudioStreamBasicDescription` containing the stream description
    pub fn fill_opus(rate: u32, channels: u32) -> Self {
        Self {
            sample_rate: rate,
            format_id: kAudioFormatOpus.try_into().expect("Failure should not happen, yet it did"),
            format_flags: 0,
            bytes_per_packet: 0,
            frames_per_packet: (rate * TryInto::<u32>::try_into(kAudioMillisecondsPerPacket_Opus).expect("There should be no failure")) / 1000,
            bytes_per_frame: 0,
            channels_per_frame: channels,
            bits_per_channel: 0,
            reserved: 0,
        }
    }
}

pub const kAirPlayAudioFormat_Invalid: u64 = 						0;
pub const kAirPlayAudioFormat_Reserved1: u64 = 					( 1 << 0 );	// 0x00000001
pub const kAirPlayAudioFormat_Reserved2: u64 = 					( 1 << 1 );	// 0x00000002
pub const kAirPlayAudioFormat_PCM_8KHz_16Bit_Mono: u64 = 		( 1 << 2 );	// 0x00000004
pub const kAirPlayAudioFormat_PCM_8KHz_16Bit_Stereo: u64 = 		( 1 << 3 );	// 0x00000008
pub const kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono: u64 = 		( 1 << 4 );	// 0x00000010
pub const kAirPlayAudioFormat_PCM_16KHz_16Bit_Stereo: u64 = 	( 1 << 5 );	// 0x00000020
pub const kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono: u64 = 		( 1 << 6 );	// 0x00000040
pub const kAirPlayAudioFormat_PCM_24KHz_16Bit_Stereo: u64 = 	( 1 << 7 );	// 0x00000080
pub const kAirPlayAudioFormat_PCM_32KHz_16Bit_Mono: u64 = 		( 1 << 8 );	// 0x00000100
pub const kAirPlayAudioFormat_PCM_32KHz_16Bit_Stereo: u64 = 	( 1 << 9 );	// 0x00000200
pub const kAirPlayAudioFormat_PCM_44KHz_16Bit_Mono: u64 = 		( 1 << 10 ); // 0x00000400
pub const kAirPlayAudioFormat_PCM_44KHz_16Bit_Stereo: u64 = 	( 1 << 11 ); // 0x00000800
pub const kAirPlayAudioFormat_PCM_44KHz_24Bit_Mono: u64 = 		( 1 << 12 ); // 0x00001000
pub const kAirPlayAudioFormat_PCM_44KHz_24Bit_Stereo: u64 = 	( 1 << 13 ); // 0x00002000
pub const kAirPlayAudioFormat_PCM_48KHz_16Bit_Mono: u64 = 		( 1 << 14 ); // 0x00004000
pub const kAirPlayAudioFormat_PCM_48KHz_16Bit_Stereo: u64 = 	( 1 << 15 ); // 0x00008000
pub const kAirPlayAudioFormat_PCM_48KHz_24Bit_Mono: u64 = 		( 1 << 16 ); // 0x00010000
pub const kAirPlayAudioFormat_PCM_48KHz_24Bit_Stereo: u64 = 	( 1 << 17 ); // 0x00020000
pub const kAirPlayAudioFormat_Reserved3: u64 = 					( 1 << 18 ); // 0x00040000
pub const kAirPlayAudioFormat_Reserved4: u64 = 					( 1 << 19 ); // 0x00080000
pub const kAirPlayAudioFormat_Reserved5: u64 = 					( 1 << 20 ); // 0x00100000
pub const kAirPlayAudioFormat_Reserved6: u64 = 					( 1 << 21 ); // 0x00200000
pub const kAirPlayAudioFormat_AAC_LC_44KHz_Stereo: u64 = 		( 1 << 22 ); // 0x00400000
pub const kAirPlayAudioFormat_AAC_LC_48KHz_Stereo: u64 = 		( 1 << 23 ); // 0x00800000
pub const kAirPlayAudioFormat_AAC_ELD_44KHz_Stereo: u64 = 		( 1 << 24 ); // 0x01000000
pub const kAirPlayAudioFormat_AAC_ELD_48KHz_Stereo: u64 = 		( 1 << 25 ); // 0x02000000
pub const kAirPlayAudioFormat_AAC_ELD_16KHz_Mono: u64 = 		( 1 << 26 ); // 0x04000000
pub const kAirPlayAudioFormat_AAC_ELD_24KHz_Mono: u64 = 		( 1 << 27 ); // 0x08000000
pub const kAirPlayAudioFormat_OPUS_16KHz_Mono: u64 = 			( 1 << 28 ); // 0x10000000
pub const kAirPlayAudioFormat_OPUS_24KHz_Mono: u64 = 			( 1 << 29 ); // 0x20000000
pub const kAirPlayAudioFormat_OPUS_48KHz_Mono: u64 = 			( 1 << 30 ); // 0x40000000
pub const kAudioFormatFlagIsFloat: u64 =						( 1 << 0 );
pub const kAudioFormatFlagIsBigEndian: u64 =					( 1 << 1 );
pub const kAudioFormatFlagIsPacked: u64 =					    ( 1 << 3 );
pub const kAudioFormatFlagIsSignedInteger: u64 =				( 1 << 2 );
pub const kAudioFormatFlagIsAlignedHigh: u64 =				    ( 1 << 4 );
pub const kAudioFormatFlagIsNonInterleaved: u64 =			    ( 1 << 5 );
pub const kAudioFormatFlagIsNonMixable: u64 =				    ( 1 << 6 );
pub const kAudioFormatLinearPCM: u64 =						    0x6C70636D; // 'lpcm'
pub const kLinearPCMFormatFlagIsFloat: u64 =					kAudioFormatFlagIsFloat;
pub const kLinearPCMFormatFlagIsBigEndian: u64 =				kAudioFormatFlagIsBigEndian;
pub const kLinearPCMFormatFlagIsSignedInteger: u64 =			kAudioFormatFlagIsSignedInteger;
pub const kLinearPCMFormatFlagIsPacked: u64 =				    kAudioFormatFlagIsPacked;
pub const kLinearPCMFormatFlagIsAlignedHigh: u64 =			    kAudioFormatFlagIsAlignedHigh;
pub const kLinearPCMFormatFlagIsNonInterleaved: u64 =		    kAudioFormatFlagIsNonInterleaved;
pub const kLinearPCMFormatFlagIsNonMixable: u64 =			    kAudioFormatFlagIsNonMixable;
pub const kLinearPCMFormatFlagsSampleFractionShift: u64 =	    7;
pub const kLinearPCMFormatFlagsSampleFractionMask: u64 =		( 0x3F << kLinearPCMFormatFlagsSampleFractionShift );
pub const kAudioFormatMPEG4AAC: u64 =						    0x61616320; // 'aac '
pub const kAudioFormatMPEG4AAC_ELD: u64 =					    0x61616365; // 'aace'
pub const kAudioFormatOpus: u64 =							    0x6F707573; // 'opus'
pub const kAudioMillisecondsPerPacket_Opus: u64 =   			20;
pub const kAudioSamplesPerPacket_AAC_ELD: u64 =		    		480;
pub const kAudioSamplesPerPacket_AAC_LC: u64 =			    	1024;
pub const kAudioFormatFlagsNativeEndian: u64 =			        0;

// Main Audio - Compatibility
pub const mac_inputFormats: u64 =
    kAirPlayAudioFormat_PCM_8KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono;
pub const mac_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_8KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_44KHz_16Bit_Stereo |
    kAirPlayAudioFormat_PCM_48KHz_16Bit_Stereo;

// Alt Audio - Compatibility
pub const aac_inputFormats: u64 = 0;
pub const aac_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_44KHz_16Bit_Stereo |
    kAirPlayAudioFormat_PCM_48KHz_16Bit_Stereo;

// Main Audio - Alert
pub const maa_inputFormats: u64 = 0;
pub const maa_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_44KHz_16Bit_Stereo |
    kAirPlayAudioFormat_PCM_48KHz_16Bit_Stereo |
    kAirPlayAudioFormat_OPUS_48KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_48KHz_Mono |
    //kAirPlayAudioFormat_AAC_ELD_44KHz_Mono |
    //kAirPlayAudioFormat_AAC_ELD_48KHz_Stereo |
    //kAirPlayAudioFormat_AAC_ELD_44KHz_Stereo;

// Main Audio - Default
pub const mad_inputFormats: u64 =
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono |
    kAirPlayAudioFormat_OPUS_24KHz_Mono |
    kAirPlayAudioFormat_OPUS_16KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_16KHz_Mono |
    //kAirPlayAudioFormat_AAC_ELD_24KHz_Mono;
pub const mad_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono |
    kAirPlayAudioFormat_OPUS_24KHz_Mono |
    kAirPlayAudioFormat_OPUS_16KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_16KHz_Mono |
    //kAirPlayAudioFormat_AAC_ELD_24KHz_Mono;

// Main Audio - Media
pub const mam_inputFormats: u64 = 0;
pub const mam_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_44KHz_16Bit_Stereo |
    kAirPlayAudioFormat_PCM_48KHz_16Bit_Stereo;

// Main Audio - Telephony
pub const mat_inputFormats: u64 =
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono |
    kAirPlayAudioFormat_OPUS_24KHz_Mono |
    kAirPlayAudioFormat_OPUS_16KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_16KHz_Mono |
    //kAirPlayAudioFormat_AAC_ELD_24KHz_Mono;
pub const mat_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono |
    kAirPlayAudioFormat_OPUS_24KHz_Mono |
    kAirPlayAudioFormat_OPUS_16KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_16KHz_Mono |
    //kAirPlayAudioFormat_AAC_ELD_24KHz_Mono;

// Main Audio - SpeechRecognition
pub const masr_inputFormats: u64 =
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_OPUS_24KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_24KHz_Mono;
pub const masr_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_OPUS_24KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_24KHz_Mono;

// Alt Audio - Default
pub const aad_inputFormats: u64 = 0;
pub const aad_outputFormats: u64 =
	kAirPlayAudioFormat_PCM_44KHz_16Bit_Stereo |
	kAirPlayAudioFormat_PCM_48KHz_16Bit_Stereo |
	kAirPlayAudioFormat_OPUS_48KHz_Mono;
	//kAirPlayAudioFormat_AAC_ELD_48KHz_Mono |
	//kAirPlayAudioFormat_AAC_ELD_44KHz_Mono |
	//kAirPlayAudioFormat_AAC_ELD_48KHz_Stereo |
	//kAirPlayAudioFormat_AAC_ELD_44KHz_Stereo;

// Main High - Media
pub const mhm_inputFormats: u64 = 0;
pub const mhm_outputFormats: u64 =
	kAirPlayAudioFormat_AAC_LC_48KHz_Stereo |
	kAirPlayAudioFormat_AAC_LC_44KHz_Stereo;

fn is_zero(v: &u64) -> bool {
    *v == 0
}

/// Equivalent to AirPlayAudioFormatToASBD, turns an AAF to ASBD
/// # Arguments
/// `format` - `u64` containing the AirPlayAudioFormat
/// 
/// # Returns
/// `AudioStreamBasicDescription` containing the stream description
pub fn audio_format_to_asbd(format: u64) -> AudioStreamBasicDescription {
    match format {
		kAirPlayAudioFormat_PCM_8KHz_16Bit_Mono =>		AudioStreamBasicDescription::fill_pcm(  8000, 16, 16, 1 ),
		kAirPlayAudioFormat_PCM_8KHz_16Bit_Stereo =>	AudioStreamBasicDescription::fill_pcm(  8000, 16, 16, 2 ),
		kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono =>		AudioStreamBasicDescription::fill_pcm( 16000, 16, 16, 1 ),
		kAirPlayAudioFormat_PCM_16KHz_16Bit_Stereo =>	AudioStreamBasicDescription::fill_pcm( 16000, 16, 16, 2 ),
		kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono =>		AudioStreamBasicDescription::fill_pcm( 24000, 16, 16, 1 ),
		kAirPlayAudioFormat_PCM_24KHz_16Bit_Stereo =>	AudioStreamBasicDescription::fill_pcm( 24000, 16, 16, 2 ),
		kAirPlayAudioFormat_PCM_32KHz_16Bit_Mono =>		AudioStreamBasicDescription::fill_pcm( 32000, 16, 16, 1 ),
		kAirPlayAudioFormat_PCM_32KHz_16Bit_Stereo =>	AudioStreamBasicDescription::fill_pcm( 32000, 16, 16, 2 ),
		kAirPlayAudioFormat_PCM_44KHz_16Bit_Mono =>		AudioStreamBasicDescription::fill_pcm( 44100, 16, 16, 1 ),
		kAirPlayAudioFormat_PCM_44KHz_16Bit_Stereo =>	AudioStreamBasicDescription::fill_pcm( 44100, 16, 16, 2 ),
		kAirPlayAudioFormat_PCM_44KHz_24Bit_Mono =>		AudioStreamBasicDescription::fill_pcm( 44100, 24, 24, 1 ),
		kAirPlayAudioFormat_PCM_44KHz_24Bit_Stereo =>	AudioStreamBasicDescription::fill_pcm( 44100, 24, 24, 2 ),
		kAirPlayAudioFormat_PCM_48KHz_16Bit_Mono =>		AudioStreamBasicDescription::fill_pcm( 48000, 16, 16, 1 ),
		kAirPlayAudioFormat_PCM_48KHz_16Bit_Stereo =>	AudioStreamBasicDescription::fill_pcm( 48000, 16, 16, 2 ),
		kAirPlayAudioFormat_PCM_48KHz_24Bit_Mono =>		AudioStreamBasicDescription::fill_pcm( 48000, 24, 24, 1 ),
		kAirPlayAudioFormat_PCM_48KHz_24Bit_Stereo =>	AudioStreamBasicDescription::fill_pcm( 48000, 24, 24, 2 ),
		kAirPlayAudioFormat_AAC_LC_44KHz_Stereo =>		AudioStreamBasicDescription::fill_aac_lc( 44100, 2 ),
		kAirPlayAudioFormat_AAC_LC_48KHz_Stereo =>		AudioStreamBasicDescription::fill_aac_lc(48000, 2 ),
		kAirPlayAudioFormat_AAC_ELD_16KHz_Mono =>		AudioStreamBasicDescription::fill_aac_eld( 16000, 1 ),
		kAirPlayAudioFormat_AAC_ELD_24KHz_Mono =>		AudioStreamBasicDescription::fill_aac_eld( 24000, 1 ),
		kAirPlayAudioFormat_AAC_ELD_44KHz_Stereo =>		AudioStreamBasicDescription::fill_aac_eld( 44100, 2 ),
		kAirPlayAudioFormat_AAC_ELD_48KHz_Stereo =>		AudioStreamBasicDescription::fill_aac_eld( 48000, 2 ),
		kAirPlayAudioFormat_OPUS_16KHz_Mono =>			AudioStreamBasicDescription::fill_opus( 16000, 1 ),
		kAirPlayAudioFormat_OPUS_24KHz_Mono =>			AudioStreamBasicDescription::fill_opus( 24000, 1 ),
		kAirPlayAudioFormat_OPUS_48KHz_Mono =>			AudioStreamBasicDescription::fill_opus( 48000, 1 ),
		kAirPlayAudioFormat_AAC_ELD_44KHz_Mono =>	AudioStreamBasicDescription::fill_aac_eld( 44100, 1 ),
		kAirPlayAudioFormat_AAC_ELD_48KHz_Mono =>	AudioStreamBasicDescription::fill_aac_eld( 48000, 1 ),
    }
}

pub fn populate_audio_formats() -> Vec<AudioFmtData> {
    let mut data: Vec<AudioFmtData> = Vec::new();

    // Main Audio - Compatibility
    data.push(AudioFmtData {
        r#type: 100,
        audioInputFormats: mac_inputFormats,
        audioOutputFormats: mac_outputFormats,
        audioType: String::from("compatibility"),
    });
    // Alt Audio - Compatibility
    data.push(AudioFmtData {
        r#type: 101,
        audioInputFormats: aac_inputFormats,
        audioOutputFormats: aac_outputFormats,
        audioType: String::from("compatibility"),
    });
    // Main Audio - Alert
    data.push(AudioFmtData {
        r#type: 100,
        audioInputFormats: maa_inputFormats,
        audioOutputFormats: maa_outputFormats,
        audioType: String::from("alert"),
    });
    // Main Audio - Default
    data.push(AudioFmtData {
        r#type: 100,
        audioInputFormats: mad_inputFormats,
        audioOutputFormats: mad_outputFormats,
        audioType: String::from("default"),
    });
    // Main Audio - Media
    data.push(AudioFmtData {
        r#type: 100,
        audioInputFormats: mam_inputFormats,
        audioOutputFormats: mam_outputFormats,
        audioType: String::from("media"),
    });
    // Main Audio - Telephony
    data.push(AudioFmtData {
        r#type: 100,
        audioInputFormats: mat_inputFormats,
        audioOutputFormats: mat_outputFormats,
        audioType: String::from("telephony"),
    });
    // Main Audio - SpeechRecognition
    data.push(AudioFmtData {
        r#type: 100,
        audioInputFormats: masr_inputFormats,
        audioOutputFormats: masr_outputFormats,
        audioType: String::from("speechRecognition"),
    });
    // Alt Audio - Default
    data.push(AudioFmtData {
        r#type: 101,
        audioInputFormats: aad_inputFormats,
        audioOutputFormats: aad_outputFormats,
        audioType: String::from("default"),
    });
    // Main High - Media
    data.push(AudioFmtData {
        r#type: 102,
        audioInputFormats: mhm_inputFormats,
        audioOutputFormats: mhm_outputFormats,
        audioType: String::from("media"),
    });
    data
}
pub fn populate_audio_latencies() -> Vec<AudioLatData> {
    let mut data: Vec<AudioLatData> = Vec::new();

    // Main Audio - Catch All
    data.push(AudioLatData {
        r#type: 100,
        audioType: None,
        inputLatencyMicros: 0,
        outputLatencyMicros: 0,
    });
    // Main Audio - Default
    data.push(AudioLatData {
        r#type: 100,
        audioType: Some(String::from("default")),
        inputLatencyMicros: 0,
        outputLatencyMicros: 0,
    });
    // Main Audio - Media
    data.push(AudioLatData {
        r#type: 100,
        audioType: Some(String::from("media")),
        inputLatencyMicros: 0,
        outputLatencyMicros: 0,
    });
    // Main Audio - Telephony
    data.push(AudioLatData {
        r#type: 100,
        audioType: Some(String::from("telephony")),
        inputLatencyMicros: 0,
        outputLatencyMicros: 0,
    });
    // Main Audio - SpeechRecognition
    data.push(AudioLatData {
        r#type: 100,
        audioType: Some(String::from("speechRecognition")),
        inputLatencyMicros: 0,
        outputLatencyMicros: 0,
    });
    // Main Audio - Alert
    data.push(AudioLatData {
        r#type: 100,
        audioType: Some(String::from("alert")),
        inputLatencyMicros: 0,
        outputLatencyMicros: 0,
    });
    // Alt Audio - Catch All
    data.push(AudioLatData {
        r#type: 101,
        audioType: None,
        inputLatencyMicros: 0,
        outputLatencyMicros: 0,
    });
    // Alt Audio - Default
    data.push(AudioLatData {
        r#type: 101,
        audioType: Some(String::from("default")),
        inputLatencyMicros: 0,
        outputLatencyMicros: 0,
    });
    // Alt Audio - Media
    data.push(AudioLatData {
        r#type: 102,
        audioType: Some(String::from("media")),
        inputLatencyMicros: 0,
        outputLatencyMicros: 0,
    });
    data
}