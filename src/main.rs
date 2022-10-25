/*
 * Calculator on an Arduino Uno
 * Created by sheepy0125
 * 2022-10-23
 */

/***** Setup *****/
#![no_std]
#![no_main]

// Imports
#[macro_use]
extern crate fixedvec;
use fixedvec::FixedVec;

use embedded_hal::serial::Read;
use nb::block;
use panic_halt as _;
use ufmt::{uWrite, uwriteln};

const MAXIMUM_LENGTH: usize = 32;

/***** Main *****/
#[arduino_hal::entry]
fn main() -> ! {
    // Handles
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut led = pins.d13.into_output();
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    // Setup
    let mut new_line_received = false;
    let mut preallocated_space = alloc_stack!([char; MAXIMUM_LENGTH]);
    let mut buffer = FixedVec::new(&mut preallocated_space);

    uwriteln!(&mut serial, "Ready for calculations!").unwrap();
    loop {
        // Fetch
        uWrite::write_str(&mut serial, "Enter an equation > ").unwrap();
        led.set_low();
        while !new_line_received {
            match block!(serial.read()).unwrap() {
                10 => new_line_received = true,
                byte => buffer
                    .push(byte.try_into().unwrap_or('?'))
                    .unwrap_or_else(|_| {
                        // The only error that this can return is one of no space left
                        new_line_received = true;
                    }),
            }
        }
        led.set_high();

        // TODO: processing

        // Output
        uWrite::write_str(&mut serial, "Received: ").unwrap();
        buffer
            .iter()
            .for_each(|char| uWrite::write_char(&mut serial, *char).unwrap());
        uWrite::write_char(&mut serial, '\n').unwrap();

        // Reset
        buffer.clear();
        new_line_received = false;
    }
}
