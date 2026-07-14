use openssl::rsa::Rsa;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use openssl::hash::MessageDigest;
use once_cell::sync::Lazy;
use std::thread;
use std::time::Duration;
use i2cdev::core::*;
use std::sync::{
    Arc,
    Mutex
};
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
fn wrapped_read_i2c(dev: &mut LinuxI2CDevice, addr: u8, n: usize) -> Result<Vec<u8>, LinuxI2CError> {
    let mut to_ret: Vec<u8> = vec![0; n];
    let read_code: &[u8] = &[0x21];
    let write_code: &[u8] = &[0x20];
    let register: &[u8] = &[addr];
    let mut msgs = [
        LinuxI2CMessage::write(register).with_address(SLAVE_ADDR),
    ];
    match dev.transfer(&mut msgs) {
        Ok(_) => {},
        Err(e) => {
            log::error!("Error while doing a wrapped i2c read! Error is: {:?}", e);
            return Err(e)
        }
    }
    let mut msgs = [
        LinuxI2CMessage::read(to_ret.as_mut_slice()).with_address(SLAVE_ADDR)
    ];
    match dev.transfer(&mut msgs) {
        Ok(_) => {},
        Err(e) => {
            log::error!("Error while doing a wrapped i2c read! Error is: {:?}", e);
            return Err(e)
        }
    }
    Ok(to_ret)
}

/// Write the Coprocessor
/// # Arguments
/// `dev1` - `Option>Arc<Mutex<LinuxI2CDevice?`
/// `addr` - `u8` containing the read address
/// `n` - `usize` containing the size to read
fn wrapped_write_i2c(dev: &mut LinuxI2CDevice, addr: u8, data: &[u8]) {
    let mut buf = Vec::with_capacity(data.len() + 1);
    buf.push(addr);
    buf.extend_from_slice(data);
    let read_code: &[u8] = &[0x21];
    let write_code: &[u8] = &[0x20];
    let mut register = vec![addr];
    register.extend_from_slice(data);
    let mut msg = [
        LinuxI2CMessage::write(register.as_slice()).with_address(SLAVE_ADDR)
    ];
    println!("{:?}", dev.transfer(&mut msg));
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
    let mut dev2;
    match dev {
        Some(lid) => dev2=lid,
        None => panic!("This shouldn't happen, but Option<&mut LinuxI2CDevice> was None...")
    }
    let mut data_read;
    let dv = String::from("device_version");
    let fv = String::from("firmware_version");
    {
        let mut devv = dev2.lock().unwrap();
        match key {
            dv => {
                data_read = wrapped_read_i2c(&mut devv, 0x00, 1);
            },
            fv => {
                data_read = wrapped_read_i2c(&mut devv, 0x01, 1);
            }
        };
    }
    match data_read {
        Ok(to_ret) => {
            return to_ret
        },
        Err(e) => {
            println!("Error while reading key! {:?}", e);
            return vec![0]
        }
    }
}

pub fn ret_cp_pubkey(dev: Option<Arc<Mutex<LinuxI2CDevice>>>) -> Vec<u8> {
    // Endianness: Big Endian (for things like: registers holding length in two bytes)
    let mut dev2;
    let mut dev1;
    match dev {
        Some(lid) => dev2=lid,
        None => {
            log::warn!("You are currently using the cp2 module for authentication, but no Coprocessor was found! Falling back to fake_mfi");
            return ret_cp_pubkey_fake()
        }
    }
    dev1 = dev2.lock().unwrap();
    let length_from_i2c = match wrapped_read_i2c(&mut dev1, 0x30, 2) {
        Ok(length) => length,
        Err(e) => {
            panic!("Error while reading Coprocessor public key! {:?}", e)
        }
    };
    let len_bytes: [u8; 2] = [length_from_i2c[0], length_from_i2c[1]];
    let length = u16::from_be_bytes(len_bytes);
    match wrapped_read_i2c(&mut dev1, 0x31, length.try_into().unwrap()) {
        Ok(pk) => pk,
        Err(e) => {
            println!("Error while reading coprocessor public key! {:?}", e);
            vec![0]
        }
    }
}

// Sign message with the embedded memory key
pub fn rsa_sign(dev: Option<Arc<Mutex<LinuxI2CDevice>>>, msg: &[u8]) -> Result<Vec<u8>, ()> {
    if dev.is_none() {
        log::warn!("You are currently using the cp2 module for authentication, but no Coprocessor was found! Falling back to fake_mfi");
        return rsa_sign_fake(msg)
    }
    println!("{:?}, {:?}", msg, msg.len());
    let mut device_uw = dev.unwrap().clone();
    let mut dev2 = device_uw.lock().unwrap();
    wrapped_write_i2c(&mut dev2, 0x21, msg);
    wrapped_write_i2c(&mut dev2, 0x10, &[0x01]);
    thread::sleep(Duration::from_millis(100));
    let mut statuscode: Vec<u8> = vec![0];
    for num in 1..10 {
        match wrapped_read_i2c(&mut dev2, 0x10, 1) {
            Ok(st) => {
                statuscode = st;
                break
            },
            Err(e) => {
                log::warn!("Error in rsa_sign: {:?}, retrying. (this could be an ordinary issue, it is not always fatal. Iteration: {})", e, num);
                thread::sleep(Duration::from_millis(100));
            }
        };
    };

    let mut datalength: usize = 0;
        for num in 1..10 {
        match wrapped_read_i2c(&mut dev2, 0x11, 2) {
            Ok(st) => {
                datalength = 64;
                println!("{:?}", st);
                break
            },
            Err(e) => {
                log::warn!("Error in rsa_sign: {:?}, retrying. (this could be an ordinary issue, it is not always fatal. Iteration: {})", e, num);
                thread::sleep(Duration::from_millis(100));
            }
        };
    };

    let mut signature: Vec<u8> = vec![];
        for num in 1..10 {
            println!("matching i2c read for rsa result");
        match wrapped_read_i2c(&mut dev2, 0x12, datalength) {
            Ok(st) => {
                signature = st;
                break
            },
            Err(e) => {
                log::warn!("Error in rsa_sign: {:?}, retrying. (this could be an ordinary issue, it is not always fatal. Iteration: {})", e, num);
                thread::sleep(Duration::from_millis(100));
            }
        };
    };

    Ok(signature)
}
