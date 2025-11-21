//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use core::cmp::min;

use defmt::*;
use defmt_rtt as _;
use embedded_hal::pwm::SetDutyCycle;
use panic_probe as _;
use rp235x_hal::clocks::init_clocks_and_plls;
use rp235x_hal::{self as hal, entry, pac, Clock};
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{StringDescriptors, UsbDeviceBuilder, UsbVidPid};
use usbd_human_interface_device::{prelude::UsbHidClassBuilder, UsbHidError};

mod hidcust;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
// use some_bsp;

/// Tell the Boot ROM about our application
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    //let core = cortex_m::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let sio = hal::Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    //let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // GPIO
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // USB HID
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USB,
        pac.USB_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut hid = UsbHidClassBuilder::new()
        .add_device(hidcust::CustomHidConfig::default())
        .build(&usb_bus);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x0001))
        .strings(&[StringDescriptors::default()
            .manufacturer("Panasonic Corporation")
            .product("Laundry LED Controller")
            .serial_number("TEST")])
        .unwrap()
        .build();

    // Initialize PWM for LED control
    //   freq = sysclk(150MHz) / ((top + 1) * div)
    let mut pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);
    let pwm_freq_khz = 100; // KHz
    let freq = clocks.system_clock.freq().to_Hz(); // Target 1kHz PWM frequency
    let top = min((freq / (pwm_freq_khz * 1000)) / 2 - 1, 65535) as u16;

    // PWM1B: GPIO3
    let pwm1 = &mut pwm_slices.pwm1;
    pwm1.set_ph_correct();
    pwm1.set_top(top);
    pwm1.set_div_int(1);
    pwm1.set_div_frac(0);
    pwm1.enable();
    let led_ir = &mut pwm1.channel_b;
    led_ir.output_to(pins.gpio3);
    led_ir.set_duty_cycle_percent(50).unwrap(); // Start with LED off

    // PWM2A: GPIO4, PWM2B: GPIO5
    let pwm2 = &mut pwm_slices.pwm2;
    pwm2.set_ph_correct();
    pwm2.set_top(top);
    pwm2.set_div_int(1);
    pwm2.set_div_frac(0);
    pwm2.enable();
    let led_wh = &mut pwm2.channel_a;
    led_wh.output_to(pins.gpio4);
    led_wh.set_duty_cycle_percent(25).unwrap(); // Start with LED off
    let led_uv = &mut pwm2.channel_b;
    led_uv.output_to(pins.gpio5);
    led_uv.set_duty_cycle_percent(75).unwrap(); // Start with LED off

    loop {
        let mut command = hidcust::CustomHidCommand::default();
        match hid.device().read_report(&mut command) {
            Ok(()) => {
                info!(
                    "Received command: wh={} ir={} uv={}",
                    command.wh, command.ir, command.uv
                );

                led_ir.set_duty_cycle_percent(min(command.ir, 100)).unwrap();
                led_wh.set_duty_cycle_percent(min(command.wh, 100)).unwrap();
                led_uv.set_duty_cycle_percent(min(command.uv, 100)).unwrap();
            }
            Err(UsbHidError::WouldBlock) => {
                // No data available, do nothing
            }
            Err(UsbHidError::SerializationError) => {
                info!("Serialization error in received command");
            }
            Err(_) => {
                info!("Unknown error in received command");
            }
        }

        if usb_dev.poll(&mut [&mut hid]) {}
    }
}

/// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [rp235x_hal::binary_info::EntryAddr; 5] = [
    rp235x_hal::binary_info::rp_cargo_bin_name!(),
    rp235x_hal::binary_info::rp_cargo_version!(),
    rp235x_hal::binary_info::rp_program_description!(c"RP2350 Template"),
    rp235x_hal::binary_info::rp_cargo_homepage_url!(),
    rp235x_hal::binary_info::rp_program_build_attribute!(),
];

// End of file
