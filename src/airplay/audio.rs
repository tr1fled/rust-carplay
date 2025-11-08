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
const kAirPlayAudioFormat_Invalid: u64 = 						0;
const kAirPlayAudioFormat_Reserved1: u64 = 					( 1 << 0 );	// 0x00000001
const kAirPlayAudioFormat_Reserved2: u64 = 					( 1 << 1 );	// 0x00000002
const kAirPlayAudioFormat_PCM_8KHz_16Bit_Mono: u64 = 		( 1 << 2 );	// 0x00000004
const kAirPlayAudioFormat_PCM_8KHz_16Bit_Stereo: u64 = 		( 1 << 3 );	// 0x00000008
const kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono: u64 = 		( 1 << 4 );	// 0x00000010
const kAirPlayAudioFormat_PCM_16KHz_16Bit_Stereo: u64 = 	( 1 << 5 );	// 0x00000020
const kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono: u64 = 		( 1 << 6 );	// 0x00000040
const kAirPlayAudioFormat_PCM_24KHz_16Bit_Stereo: u64 = 	( 1 << 7 );	// 0x00000080
const kAirPlayAudioFormat_PCM_32KHz_16Bit_Mono: u64 = 		( 1 << 8 );	// 0x00000100
const kAirPlayAudioFormat_PCM_32KHz_16Bit_Stereo: u64 = 	( 1 << 9 );	// 0x00000200
const kAirPlayAudioFormat_PCM_44KHz_16Bit_Mono: u64 = 		( 1 << 10 ); // 0x00000400
const kAirPlayAudioFormat_PCM_44KHz_16Bit_Stereo: u64 = 	( 1 << 11 ); // 0x00000800
const kAirPlayAudioFormat_PCM_44KHz_24Bit_Mono: u64 = 		( 1 << 12 ); // 0x00001000
const kAirPlayAudioFormat_PCM_44KHz_24Bit_Stereo: u64 = 	( 1 << 13 ); // 0x00002000
const kAirPlayAudioFormat_PCM_48KHz_16Bit_Mono: u64 = 		( 1 << 14 ); // 0x00004000
const kAirPlayAudioFormat_PCM_48KHz_16Bit_Stereo: u64 = 	( 1 << 15 ); // 0x00008000
const kAirPlayAudioFormat_PCM_48KHz_24Bit_Mono: u64 = 		( 1 << 16 ); // 0x00010000
const kAirPlayAudioFormat_PCM_48KHz_24Bit_Stereo: u64 = 	( 1 << 17 ); // 0x00020000
const kAirPlayAudioFormat_Reserved3: u64 = 					( 1 << 18 ); // 0x00040000
const kAirPlayAudioFormat_Reserved4: u64 = 					( 1 << 19 ); // 0x00080000
const kAirPlayAudioFormat_Reserved5: u64 = 					( 1 << 20 ); // 0x00100000
const kAirPlayAudioFormat_Reserved6: u64 = 					( 1 << 21 ); // 0x00200000
const kAirPlayAudioFormat_AAC_LC_44KHz_Stereo: u64 = 		( 1 << 22 ); // 0x00400000
const kAirPlayAudioFormat_AAC_LC_48KHz_Stereo: u64 = 		( 1 << 23 ); // 0x00800000
const kAirPlayAudioFormat_AAC_ELD_44KHz_Stereo: u64 = 		( 1 << 24 ); // 0x01000000
const kAirPlayAudioFormat_AAC_ELD_48KHz_Stereo: u64 = 		( 1 << 25 ); // 0x02000000
const kAirPlayAudioFormat_AAC_ELD_16KHz_Mono: u64 = 		( 1 << 26 ); // 0x04000000
const kAirPlayAudioFormat_AAC_ELD_24KHz_Mono: u64 = 		( 1 << 27 ); // 0x08000000
const kAirPlayAudioFormat_OPUS_16KHz_Mono: u64 = 			( 1 << 28 ); // 0x10000000
const kAirPlayAudioFormat_OPUS_24KHz_Mono: u64 = 			( 1 << 29 ); // 0x20000000
const kAirPlayAudioFormat_OPUS_48KHz_Mono: u64 = 			( 1 << 30 ); // 0x40000000

// Main Audio - Compatibility
const mac_inputFormats: u64 =
    kAirPlayAudioFormat_PCM_8KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono;
const mac_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_8KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_44KHz_16Bit_Stereo |
    kAirPlayAudioFormat_PCM_48KHz_16Bit_Stereo;

// Alt Audio - Compatibility
const aac_inputFormats: u64 = 0;
const aac_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_44KHz_16Bit_Stereo |
    kAirPlayAudioFormat_PCM_48KHz_16Bit_Stereo;

// Main Audio - Alert
const maa_inputFormats: u64 = 0;
const maa_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_44KHz_16Bit_Stereo |
    kAirPlayAudioFormat_PCM_48KHz_16Bit_Stereo |
    kAirPlayAudioFormat_OPUS_48KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_48KHz_Mono |
    //kAirPlayAudioFormat_AAC_ELD_44KHz_Mono |
    //kAirPlayAudioFormat_AAC_ELD_48KHz_Stereo |
    //kAirPlayAudioFormat_AAC_ELD_44KHz_Stereo;

// Main Audio - Default
const mad_inputFormats: u64 =
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono |
    kAirPlayAudioFormat_OPUS_24KHz_Mono |
    kAirPlayAudioFormat_OPUS_16KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_16KHz_Mono |
    //kAirPlayAudioFormat_AAC_ELD_24KHz_Mono;
const mad_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono |
    kAirPlayAudioFormat_OPUS_24KHz_Mono |
    kAirPlayAudioFormat_OPUS_16KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_16KHz_Mono |
    //kAirPlayAudioFormat_AAC_ELD_24KHz_Mono;

// Main Audio - Media
const mam_inputFormats: u64 = 0;
const mam_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_44KHz_16Bit_Stereo |
    kAirPlayAudioFormat_PCM_48KHz_16Bit_Stereo;

// Main Audio - Telephony
const mat_inputFormats: u64 =
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono |
    kAirPlayAudioFormat_OPUS_24KHz_Mono |
    kAirPlayAudioFormat_OPUS_16KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_16KHz_Mono |
    //kAirPlayAudioFormat_AAC_ELD_24KHz_Mono;
const mat_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_PCM_16KHz_16Bit_Mono |
    kAirPlayAudioFormat_OPUS_24KHz_Mono |
    kAirPlayAudioFormat_OPUS_16KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_16KHz_Mono |
    //kAirPlayAudioFormat_AAC_ELD_24KHz_Mono;

// Main Audio - SpeechRecognition
const masr_inputFormats: u64 =
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_OPUS_24KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_24KHz_Mono;
const masr_outputFormats: u64 =
    kAirPlayAudioFormat_PCM_24KHz_16Bit_Mono |
    kAirPlayAudioFormat_OPUS_24KHz_Mono;
    //kAirPlayAudioFormat_AAC_ELD_24KHz_Mono;

// Alt Audio - Default
const aad_inputFormats: u64 = 0;
const aad_outputFormats: u64 =
	kAirPlayAudioFormat_PCM_44KHz_16Bit_Stereo |
	kAirPlayAudioFormat_PCM_48KHz_16Bit_Stereo |
	kAirPlayAudioFormat_OPUS_48KHz_Mono;
	//kAirPlayAudioFormat_AAC_ELD_48KHz_Mono |
	//kAirPlayAudioFormat_AAC_ELD_44KHz_Mono |
	//kAirPlayAudioFormat_AAC_ELD_48KHz_Stereo |
	//kAirPlayAudioFormat_AAC_ELD_44KHz_Stereo;

// Main High - Media
const mhm_inputFormats: u64 = 0;
const mhm_outputFormats: u64 =
	kAirPlayAudioFormat_AAC_LC_48KHz_Stereo |
	kAirPlayAudioFormat_AAC_LC_44KHz_Stereo;

fn is_zero(v: &u64) -> bool {
    *v == 0
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