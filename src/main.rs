/*
 * Calculator on an Arduino Uno
 * Created by sheepy0125
 * 2022-10-23
 */

/***** Setup *****/
#![no_std]
#![no_main]
// Imports
use panic_halt as _;
// Constants
const INCREASE_DELAY_BY: u16 = 10;

/***** Main *****/
#[arduino_hal::entry]
fn main() -> ! {
    // Handles
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut led = pins.d13.into_output();

    // Blink with an ever-increasing delay
    let mut delay_ms = 0;
    loop {
        led.toggle();
        arduino_hal::delay_ms(match led.is_set_high() {
            true => 50,
            false => {
                delay_ms += INCREASE_DELAY_BY; // pro tip: do this less than 6554 times
                delay_ms
            }
        });

        if delay_ms >= 1000 {
            panic!("Delay was too much!");
        }
    }
}
