use openssl::rsa::Rsa;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use openssl::hash::MessageDigest;
use once_cell::sync::Lazy;
use std::thread;
use std::time::Duration;
use i2cdev::core::*;
use crate::mfi::fake_mfi::ret_cp_pubkey as ret_cp_pubkey_fake;
use crate::mfi::fake_mfi::rsa_sign as rsa_sign_fake;
#[cfg(any(target_os = "linux", target_os = "android"))]
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError, LinuxI2CMessage};

const SLAVE_ADDR: u16 = 0x10;

/// Read the Coprocessor
/// # Arguments
/// `dev1` - `Option>Arc<Mutex<LinuxI2CDevice?`
/// `addr` - `u8` containing the read address
/// `n` - `usize` containing the size to read
fn wrapped_read_i2c(dev: &mut LinuxI2CDevice, addr: u8, n: usize) -> Vec<u8> {
    let mut to_ret: Vec<u8> = Vec::with_capacity(n);
    let mut msgs = [
    LinuxI2CMessage::write(&[addr]).with_address(SLAVE_ADDR),
    LinuxI2CMessage::read(to_ret.as_mut_slice()).with_address(SLAVE_ADDR)
    ];
    match dev.transfer(&mut msgs) {
        Ok(_) => {},
        Err(e) => {
            log::error!("Error while doing a wrapped i2c read! Error is: {:?}", e);
            to_ret = vec![0]
        }
    }
    
    to_ret
}

/// Write the Coprocessor
/// # Arguments
/// `dev1` - `Option>Arc<Mutex<LinuxI2CDevice?`
/// `addr` - `u8` containing the read address
/// `n` - `usize` containing the size to read
fn wrapped_write_i2c(dev1: Option<Arc<Mutex<LinuxI2CDevice>>>, addr: u8, data: Vec<u8>) {
    let mut dev;
    match dev1 {
        Some(lid) => dev=lid,
        None => panic!("This shouldn't happen, but Option<&mut LinuxI2CDevice> was None...")
    }
    {
        dev = dev.lock().unwrap();
        dev.smbus_write_i2c_block_data(addr, data.as_slice());
    }
}

fn wrapped_read_byte(dev: &mut LinuxI2CDevice, register: u8) -> u8 {
    match dev.smbus_read_byte_data(register) {
        Ok(data) => data,
        Err(error) => {
            log::error!("Ran into error while reading Coprocessor:\nRegister: {}\nError: {:?}", register, error);
            0x00
        }
    }
}

pub fn read_key(dev: Option<Arc<Mutex<LinuxI2CDevice>>>, key: String) -> Vec<u8> {
    let mut dev1;
    match dev {
        Some(lid) => dev1=lid,
        None => panic!("This shouldn't happen, but Option<&mut LinuxI2CDevice> was None...")
    }
    let mut data_read;
    let dv = String::from("device_version");
    let fv = String::from("firmware_version");
    {
        dev1 = dev1.lock().unwrap();
        match key {
            dv => {
                data_read = dev1.smbus_read_byte_data(0x00);
            },
            fv => {
                data_read = dev1.smbus_read_byte_data(0x01);
            }
        };
    }
    data_read.vec
}

pub fn ret_cp_pubkey(dev: Option<Arc<Mutex<LinuxI2CDevice>>>) -> Vec<u8> {
    // Endianness: Big Endian (for things like: registers holding length in two bytes)
    let mut dev1;
    match dev {
        Some(lid) => dev1=lid,
        None => {
            log::warn!("You are currently using the cp2 module for authentication, but no Coprocessor was found! Falling back to fake_mfi");
            return ret_cp_pubkey_fake()
        }
    }
    dev1 = dev1.lock().unwrap();
    let length_from_i2c = wrapped_read_i2c(dev1, 0x30, 2);
    let len_bytes: [u8; 2] = [length_from_i2c[0], length_from_i2c[1]];
    let length = u16::from_be_bytes(len_bytes);
    wrapped_read_i2c(dev1, 0x31, length.try_into().unwrap())
}

// Sign message with the embedded memory key
pub fn rsa_sign(dev: Option<Arc<Mutex<LinuxI2CDevice>>>, msg: &[u8]) -> Result<Vec<u8>, ()> {
    let mut dev2;
    let mut dev1;
    match dev {
        Some(lid) => dev1=lid,
        None => {
            log::warn!("You are currently using the cp2 module for authentication, but no Coprocessor was found! Falling back to fake_mfi");
            return rsa_sign_fake(msg)
        }
    }
    dev2 = dev1.lock().unwrap();
    let len = msg.len().to_be_bytes();
    let final_len: Vec<u8> = vec![len[0], len[1]];
    wrapped_write_i2c(dev2, 0x20, final_len);
    wrapped_write_i2c(dev2, 0x21, msg.to_vec());
    let mut msgs = [
    LinuxI2CMessage::write(&[addr]).with_address(SLAVE_ADDR),
    ];
    dev2.smbus_write_byte_data(0x10, 0x01);
    thread::sleep(Duration::from_millis(1));
    for num in 0..10 {
        let bit = match dev2.smbus_read_byte_data(0x10) {
            Ok(data) => {
                if data == 0x10 { break }
                thread::sleep(Duration::from_millis(10));
                data
            },
            Err(e) => panic!("Error while reading Coprocessor: {:?}", e),
        };
        if num == 9 {
            panic!("Timeout while reading Coprocessor, the last bit on register 0x10 (Authentication Control and Status) was: {}", bit);
        }
    };
    log::info!("Coprocessor successfully signed challenge!");
    let size = wrapped_read_i2c(dev2, 0x11, 2);
    let size_translation: [u8; 2] = [size[0], size[1]];
    let final_size = u16::from_be_bytes(size_translation);
    Ok(wrapped_read_i2c(dev2, 0x12, final_size.try_into().unwrap()))
}
