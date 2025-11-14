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
    
    // Padding (7 bits constant)
    0x75, 0x07,
    0x95, 0x01,
    0x81, 0x01,
    
    // X position
    0x05, 0x01,         // Usage Page (Generic Desktop)
    0x09, 0x30,         // Usage (X)
    0x15, 0x00,         // Logical Minimum (0)
    0x26, 0xFF, 0x7F,   // Logical Maximum (width placeholder)
    0x75, 0x10,         // Report Size (16)
    0x95, 0x01,         // Report Count (1)
    0x81, 0x02,         // Input (Data, Variable, Absolute)
    
    // Y position
    0x09, 0x31,         // Usage (Y)
    0x15, 0x00,         // Logical Minimum (0)
    0x26, 0xFF, 0x7F,   // Logical Maximum (height placeholder)
    0x75, 0x10,         // Report Size (16)
    0x95, 0x01,         // Report Count (1)
    0x81, 0x02,         // Input (Data, Variable, Absolute)
    
    0xC0,               // End Collection (Logical)
    0xC0,               // End Collection (Application)
    ];
    
    // Patch width and height into the correct positions
    // (same offsets as in the C code)
    desc[0x27] = (width & 0xFF) as u8;
    desc[0x28] = (width >> 8) as u8;
    desc[0x34] = (height & 0xFF) as u8;
    desc[0x35] = (height >> 8) as u8;
    
    desc
}

pub fn hid_touchscreen_fill_report(pressing: bool, inX: u16, inY: u16) -> [u8; 5] {
    [
        pressing as u8,
        inX as u8,
        (inX >> 8) as u8,
        inY as u8,
        (inY >> 8) as u8,
    ]
}