//! hidcust.rs
//! Custom HID class implementation for USB communication
//! using the usbd-human-interface-device crate.
//! Copyright 2025 Panasonic Corporation. All rights reserved.

use core::{default::Default, result::Result::Ok};
use fugit::ExtU32;
use packed_struct::prelude::*;
use usb_device::bus::{UsbBus, UsbBusAllocator};
use usbd_human_interface_device::{
    descriptor::InterfaceProtocol, device::*, interface::*, prelude::*,
};

macro_rules! unwrap {
    ($arg:expr) => {
        match $arg {
            ::core::result::Result::Ok(val) => val,
            ::core::result::Result::Err(e) => {
                ::core::panic!("unwrap of `{}` failed: {:?}", ::core::stringify!($arg), e);
            }
        }
    };
    ($arg:expr, $($msg:expr),+$(,)?) => {
        match $arg {
            ::core::result::Result::Ok(val) => val,
            ::core::result::Result::Err(_e) => {
                ::core::panic!("unwrap of `{}` failed: {}", ::core::stringify!($arg), ::core::format_args!($($msg),+));
            }
        }
    }
}

macro_rules! error {
    ($s:literal $(, $x:expr)* $(,)?) => {
        let _ = ($( & $x ),*);
    };
}

#[rustfmt::skip]
pub const CUSTOM_HID_REPORT_DESCRIPTOR: &[u8] = &[
    0x06, 0x00, 0xff,       // USAGE_PAGE (Vendor Defined Page 1)
    0x09, 0x01,             // USAGE (Vendor Usage 1)
    0xa1, 0x01,             // COLLECTION (Application)
    // Input Report: Device -> Host (4 bytes)
    0x15, 0x00,             //   LOGICAL_MINIMUM (0)
    0x26, 0xff, 0x00,       //   LOGICAL_MAXIMUM (255)
    0x75, 0x08,             //   REPORT_SIZE (8)
    0x95, 0x04,             //   REPORT_COUNT (4)
	0x09, 0x01,			    //   USAGE (Vendor Usage 1)
    0x81, 0x02,             //   INPUT (Data,Var,Abs)
    // Output Report: Host -> Device (4 bytes)
    0x15, 0x00,             //   LOGICAL_MINIMUM (0)
    0x26, 0xff, 0x00,       //   LOGICAL_MAXIMUM (255)
    0x75, 0x08,             //   REPORT_SIZE (8)
    0x95, 0x04,             //   REPORT_COUNT (4)
	0x09, 0x01,			    //   USAGE (Vendor Usage 1)
    0x91, 0x02,             //   OUTPUT (Data,Var,Abs)
    0xc0                    // END_COLLECTION   
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default, PackedStruct)]
#[packed_struct(endian = "lsb")]
pub struct CustomHidReport {
    #[packed_field]
    pub data: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default, PackedStruct)]
#[packed_struct(endian = "lsb")]
pub struct CustomHidCommand {
    #[packed_field]
    pub wh: u8, // duty percent for white led
    #[packed_field]
    pub ir: u8, // duty percent for infrared led
    #[packed_field]
    pub uv: u8, // duty percent for ultraviolet led
    #[packed_field]
    pub reserved: u8,
}

pub struct CustomHid<'a, B: UsbBus> {
    interface: Interface<'a, B, InBytes8, OutBytes8, ReportSingle>,
}

#[allow(dead_code)]
impl<B: UsbBus> CustomHid<'_, B> {
    pub fn write_report(&mut self, report: &CustomHidReport) -> Result<(), UsbHidError> {
        let data = report.pack().map_err(|_| {
            error!("Error packing CustomHidReport");
            UsbHidError::SerializationError
        })?;
        self.interface
            .write_report(&data)
            .map(|_| ())
            .map_err(UsbHidError::from)
    }

    pub fn read_report(&mut self, command: &mut CustomHidCommand) -> Result<(), UsbHidError> {
        let mut buf = [0u8; 4];
        self.interface
            .read_report(&mut buf)
            .map_err(UsbHidError::from)?;
        let cmd = CustomHidCommand::unpack(&buf).map_err(|_| {
            error!("Error unpacking CustomHidCommand");
            UsbHidError::SerializationError
        })?;
        *command = cmd;
        Ok(())
    }
}

pub struct CustomHidConfig<'a> {
    interface: InterfaceConfig<'a, InBytes8, OutBytes8, ReportSingle>,
}

impl<'a> CustomHidConfig<'a> {
    #[must_use]
    pub fn new(interface: InterfaceConfig<'a, InBytes8, OutBytes8, ReportSingle>) -> Self {
        Self { interface }
    }
}

impl Default for CustomHidConfig<'_> {
    fn default() -> Self {
        CustomHidConfig::new(
            unwrap!(unwrap!(InterfaceBuilder::new(CUSTOM_HID_REPORT_DESCRIPTOR))
                .boot_device(InterfaceProtocol::None)
                .description("Custom HID Interface")
                .in_endpoint(10.millis()))
            .with_out_endpoint(10.millis())
            .unwrap()
            .build(),
        )
    }
}

impl<'a, B: UsbBus + 'a> UsbAllocatable<'a, B> for CustomHidConfig<'a> {
    type Allocated = CustomHid<'a, B>;

    fn allocate(self, usb_alloc: &'a UsbBusAllocator<B>) -> Self::Allocated {
        CustomHid {
            interface: self.interface.allocate(usb_alloc),
        }
    }
}

impl<'a, B: UsbBus> DeviceClass<'a> for CustomHid<'a, B> {
    type I = Interface<'a, B, InBytes8, OutBytes8, ReportSingle>;

    fn interface(&mut self) -> &mut Self::I {
        &mut self.interface
    }

    fn reset(&mut self) {}

    fn tick(&mut self) -> Result<(), UsbHidError> {
        Ok(())
    }
}
