/* SPDX-License-Identifier: GPL-3.0-or-later */

//! Development probe only: no daemon installation or input grabbing.
//! Default is read-only; --test-leds enables a bounded three-second test.

#[path = "../src/hwdevices/roccat_vulcan_2_protocol.rs"]
mod protocol;

use hidapi::{HidApi, HidDevice};
use protocol::{encode_direct, DeviceInfo};
use std::{error::Error, thread, time::Duration};

fn discover(api: &HidApi, interface: i32, usage_page: u16) -> Result<HidDevice, Box<dyn Error>> {
    let matches: Vec<_> = api
        .device_list()
        .filter(|device| {
            device.vendor_id() == 0x1e7d
                && device.product_id() == 0x2f4e
                && device.interface_number() == interface
                && device.usage_page() == usage_page
        })
        .collect();
    if matches.len() != 1 {
        return Err(format!(
            "Expected exactly one Vulcan II interface {interface}, found {}",
            matches.len()
        )
        .into());
    }
    println!("Interface {interface}: {:?}", matches[0].path());
    Ok(matches[0].open_device(api)?)
}

struct DirectMode<'a>(&'a HidDevice);

impl Drop for DirectMode<'_> {
    fn drop(&mut self) {
        match self.0.send_feature_report(&[0x0e, 0x05, 0, 0, 0]) {
            Ok(()) => eprintln!("Direct mode disabled."),
            result => eprintln!("WARNING: Failed to disable Direct mode: {result:?}. Unplug/replug the keyboard if necessary."),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let test_leds = match args.as_slice() {
        [] => false,
        [arg] if arg == "--test-leds" => true,
        _ => return Err("Usage: vulcan_2_probe [--test-leds]".into()),
    };
    let api = HidApi::new()?;
    for device in api
        .device_list()
        .filter(|d| d.vendor_id() == 0x1e7d && d.product_id() == 0x2f4e)
    {
        println!(
            "HID {:?}: interface {}, usage page 0x{:04x}, usage 0x{:04x}",
            device.path(),
            device.interface_number(),
            device.usage_page(),
            device.usage()
        );
    }
    let ctrl = discover(&api, 1, 0xff01)?;
    let mut report = [0u8; 9];
    report[0] = 9;
    let length = ctrl.get_feature_report(&mut report)?;
    let info = DeviceInfo::parse(&report[..length])?;
    println!(
        "Vulcan II: firmware {}.{:02}, layout 0x{:02x}, length order {:?}",
        info.firmware_major,
        info.firmware_minor,
        info.layout,
        info.length_byte_order()
    );
    if !test_leds {
        println!("Read-only probe complete; no lighting writes.");
        return Ok(());
    }
    if (info.firmware_major, info.firmware_minor, info.layout) != (1, 9, 1) {
        return Err("LED probe currently validated only for firmware 1.09, German layout".into());
    }
    // Avoid ambiguity for multiple keyboards without unique serial numbers.
    let led = discover(&api, 3, 0xff00)?;
    // OpenRGB ISO mapping: Escape=2, F1=13, F2=20. Dim output.
    let packets = encode_direct(
        &[(2, [32, 0, 0]), (13, [0, 32, 0]), (20, [0, 0, 32])],
        info.length_byte_order(),
    )?;
    println!("Three-second test: Escape red, F1 green, F2 blue; other LEDs black.");
    // Install cleanup before enabling, so even a partial enable failure
    // attempts to restore the firmware-controlled lighting mode.
    let _direct = DirectMode(&ctrl);
    ctrl.send_feature_report(&[0x0e, 0x05, 1, 0, 0])?;
    for packet in &packets {
        if led.write(packet)? != packet.len() {
            return Err("Short LED output write".into());
        }
    }
    thread::sleep(Duration::from_secs(3));
    Ok(())
}
