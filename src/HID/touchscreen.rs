use serde::Serialize;
use std::io::Cursor;

#[derive(Serialize)]
struct hid_report {
    pub r#type: String,
    pub uuid: String,
    pub hidReport: plist::Data,
}

impl hid_report {
    pub fn new(uuid: String, report: Vec<u8>) -> Self {
        Self {
            r#type: String::from("hidSendReport"),
            uuid,
            hidReport: plist::Data::new(report),
        }
    }
} 

pub fn hid_touchscreen_single_descriptor(width: u16, height: u16) -> Vec<u8> {
    let mut desc: Vec<u8> = vec![
        0x05, 0x0D,         // Usage Page (Digitizer)
        0x09, 0x04,         // Usage (Touch Screen)
        0xA1, 0x01,         // Collection (Application)

        0x05, 0x0D,         // Usage Page (Digitizer)
        0x09, 0x22,         // Usage (Finger)
        0xA1, 0x02,         // Collection (Logical)

        // Touch
        0x05, 0x0D,         // Usage Page (Digitizer)
        0x09, 0x33,         // Usage (Touch)
        0x15, 0x00,         // Logical Minimum (0)
        0x25, 0x01,         // Logical Maximum (1)
        0x75, 0x01,         // Report Size (1)
        0x95, 0x01,         // Report Count (1)
        0x81, 0x02,         // Input (Data, Variable, Absolute)

        // Padding
        0x75, 0x07,
        0x95, 0x01,
        0x81, 0x01,

        // X
        0x05, 0x01,         // Usage Page (Generic Desktop)
        0x09, 0x30,         // Usage (X)
        0x15, 0x00,         // Logical Minimum
        0x26, 0xFF, 0x7F,   // Logical Maximum (placeholder)
        0x75, 0x10,
        0x95, 0x01,
        0x81, 0x02,

        // Y
        0x09, 0x31,         // Usage (Y)
        0x15, 0x00,
        0x26, 0xFF, 0x7F,   // Logical Maximum (placeholder)
        0x75, 0x10,
        0x95, 0x01,
        0x81, 0x02,

        0xC0,
        0xC0,
    ];

    // -------------------------------------------------------------------
    // 🔧 Patch the width + height dynamically by FINDING the right places
    // -------------------------------------------------------------------

    // Find FIRST 0x26 ff 7f  → X logical maximum
    if let Some(pos) = desc.windows(3).position(|w| w == [0x26, 0xFF, 0x7F]) {
        desc[pos + 1] = (width & 0xFF) as u8;
        desc[pos + 2] = (width >> 8) as u8;
    }

    // Find SECOND 0x26 ff 7f → Y logical maximum
    if let Some(pos2) = desc
        .windows(3)
        .enumerate()
        .filter(|(i, w)| *w == [0x26, 0xFF, 0x7F] && *i > 0)
        .map(|(i, _)| i)
        .nth(1)
    {
        desc[pos2 + 1] = (height & 0xFF) as u8;
        desc[pos2 + 2] = (height >> 8) as u8;
    }

    desc
}

pub fn hid_touchscreen_fill_report(pressing: bool, inX: u16, inY: u16) -> Vec<u8> {
    let mut ret: Vec<u8> = Vec::new();
    let report: Vec<u8> = vec![
        pressing as u8,   // 1 or 0
        inX as u8,        // X pos
        (inX >> 8) as u8, // X pos RoR
        inY as u8,        // Y pos
        (inY >> 8) as u8, // Y pos RoR
    ];
    // generate a new HID report and turn it to a bplist
    plist::to_writer_binary(&mut ret, &hid_report::new(String::from("e5f7a68d-7b0f-4305-984b-974f677a150b"), report));
    ret
}