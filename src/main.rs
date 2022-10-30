/*!
 * Calculator on an Arduino Uno
 * Created by sheepy0125
 * 2022-10-23
 */

/***** Setup *****/
#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

// Imports
#[macro_use]
extern crate fixedvec;

use arduino_hal::prelude::*;
use embedded_hal::serial::Read;
use fixedvec::FixedVec;
use nb::block;

// Constants
const MAXIMUM_LENGTH: usize = 32;
const BAUD_RATE: u32 = 57600;

/***** Panic handler *****/
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Let's steal our handlers
    let dp = unsafe { arduino_hal::Peripherals::steal() };
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, BAUD_RATE);

    // Print out panic location
    match info.location() {
        #[cfg(not(debug_assertions))]
        Some(loc) => ufmt::uwriteln!(
            &mut serial,
            "PANICKED {}:{}:{}",
            loc.file(),
            loc.line(),
            loc.column()
        )
        .void_unwrap(),
        #[cfg(debug_assertions)]
        Some(loc) => ufmt::uwriteln!(
            &mut serial,
            "PANICKED: not release mode, garbage: {}",
            loc.file()
        )
        .void_unwrap(),
        None => ufmt::uwriteln!(&mut serial, "Panicked!").void_unwrap(),
    }

    // Blink LED rapidly
    let mut led = pins.d13.into_output();
    loop {
        led.toggle();
        arduino_hal::delay_ms(500);
    }
}

/***** Enums *****/
/// Parse stage
enum ParseStage {
    FirstNumber(u8),
    Operator(MathOperator),
    SecondNumber(u8),
    ParseError,
}

/// Operator
enum MathOperator {
    // Four-function
    /// Add two numbers
    Addition,
    /// Multiply two numbers
    Multiplication,
    /// Subtract two numbers
    Subtraction,
    /// Divide two numbers
    Division,
    // Comparison
    /// Return the number that is greater
    GreaterThan,
    /// Return the number that is lesser
    LessThan,
    // Extra
    /// Multiply the first number to a power
    Power,
    /// Get the remainder of a division
    Modulo,
}
impl MathOperator {
    fn new(char: char) -> Result<Self, ()> {
        use MathOperator::*;
        match char {
            '+' => Ok(Addition),
            '*' => Ok(Multiplication),
            '-' => Ok(Subtraction),
            '/' => Ok(Division),
            '>' => Ok(GreaterThan),
            '<' => Ok(LessThan),
            '^' => Ok(Power),
            '%' => Ok(Modulo),
            _ => Err(()),
        }
    }
    fn operate(&self, first_number: i64, second_number: i64) -> i64 {
        use MathOperator::*;
        match self {
            Addition => first_number + second_number,
            Multiplication => first_number * second_number,
            Subtraction => first_number - second_number,
            Division => first_number / second_number,
            GreaterThan => match first_number > second_number {
                true => first_number,
                false => second_number,
            },
            LessThan => match first_number > second_number {
                false => first_number,
                true => second_number,
            },
            Power => first_number.pow(second_number.try_into().unwrap_or(1)),
            Modulo => first_number % second_number,
        }
    }
}
impl Default for MathOperator {
    fn default() -> Self {
        Self::Addition
    }
}

/***** Main *****/
#[arduino_hal::entry]
fn main() -> ! {
    // Handles
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, BAUD_RATE);

    // Setup
    let mut new_line_received = false;
    let mut preallocated_space = alloc_stack!([char; MAXIMUM_LENGTH]);
    let mut buffer = FixedVec::new(&mut preallocated_space);

    // Main loop
    loop {
        // Fetch equation
        ufmt::uWrite::write_str(&mut serial, "Enter an equation > ").void_unwrap();
        while !new_line_received {
            match block!(serial.read())
                .void_unwrap()
                .try_into()
                .unwrap_or_else(|_| {
                    ufmt::uwriteln!(
                        &mut serial,
                        "Could not convert serial input into a char, substituting '?'",
                    )
                    .void_unwrap();
                    '?'
                }) {
                // Newline signifies end of equation
                '\n' => new_line_received = true,
                // Ignore spaces
                ' ' => {}
                // Everything else
                byte => buffer.push(byte).unwrap_or_else(|_| {
                    // The only error that this can return is one of no space left
                    new_line_received = true;
                }),
            }
        }

        // Parse
        use ParseStage::*;
        let mut valid = true;
        let mut first_number = None;
        let mut second_number = None;
        let mut operator = None;
        for char in buffer.iter() {
            // Get the stage
            let stage = match char.is_ascii_digit() {
                // We got a digit, check if we're getting the 1st or 2nd number
                // depending on if the operator is set
                true => {
                    // Get the number
                    // Safety: We know the character is a digit between 0-9, so it will fit into a u8
                    let number: u8 = char.to_digit(10).unwrap().try_into().unwrap();
                    match operator {
                        None => ParseStage::FirstNumber(number),
                        Some(_) => SecondNumber(number),
                    }
                }
                // We don't have a digit, so that must mean we have an operator
                false => {
                    // Sanity check
                    if first_number.is_none() || second_number.is_some() {
                        ParseError
                    } else {
                        match MathOperator::new(*char) {
                            Ok(operator) => Operator(operator),
                            Err(_) => ParseError,
                        }
                    }
                }
            };

            // Handle the stage
            match stage {
                ParseError => {
                    valid = false;
                    break;
                }
                FirstNumber(number) => {
                    first_number = Some(first_number.unwrap_or(0) * 10 + number as i64);
                }
                Operator(math_operator) => operator = Some(math_operator),
                SecondNumber(number) => {
                    second_number = Some(second_number.unwrap_or(0) * 10 + number as i64);
                }
            };
        }

        match valid {
            true => {
                // Safety: This can't be None if parsing was valid
                let first_number = first_number.unwrap();
                let operator = operator.unwrap();
                let second_number = second_number.unwrap();

                let answer = operator.operate(first_number, second_number);
                ufmt::uwriteln!(&mut serial, "RESULT: {}", answer).void_unwrap();
            }
            false => {
                ufmt::uwriteln!(&mut serial, "ERROR").void_unwrap();
            }
        }

        // Reset
        buffer.clear();
        new_line_received = false;
    }
}
