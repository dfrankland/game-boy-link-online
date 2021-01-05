mod printer;

use gpio_cdev::{Chip, Line};
use lazy_static::lazy_static;
use rppal::system::{DeviceInfo, Model};
use std::{env, error::Error, time::Duration};

lazy_static! {
    // Game Boy clock (SCK) speed is 8192 Hz
    static ref GAME_BOY_CLOCK_SPEED: Duration = {
        Duration::from_secs(1) / 8192
    };
}

#[derive(Debug, Clone, Copy)]
enum PinType {
    Gpio(u8),
    Ground,
    Power3v3,
    Power5v,
}

const HEADER: [PinType; 40] = [
    PinType::Power3v3, // Physical pin 1
    PinType::Power5v,  // Physical pin 2
    PinType::Gpio(2),  // Physical pin 3
    PinType::Power5v,  // Physical pin 4
    PinType::Gpio(3),  // Physical pin 5
    PinType::Ground,   // Physical pin 6
    PinType::Gpio(4),  // Physical pin 7
    PinType::Gpio(14), // Physical pin 8
    PinType::Ground,   // Physical pin 9
    PinType::Gpio(15), // Physical pin 10
    PinType::Gpio(17), // Physical pin 11
    PinType::Gpio(18), // Physical pin 12
    PinType::Gpio(27), // Physical pin 13
    PinType::Ground,   // Physical pin 14
    PinType::Gpio(22), // Physical pin 15
    PinType::Gpio(23), // Physical pin 16
    PinType::Power3v3, // Physical pin 17
    PinType::Gpio(24), // Physical pin 18
    PinType::Gpio(10), // Physical pin 19
    PinType::Ground,   // Physical pin 20
    PinType::Gpio(9),  // Physical pin 21
    PinType::Gpio(25), // Physical pin 22
    PinType::Gpio(11), // Physical pin 23
    PinType::Gpio(8),  // Physical pin 24
    PinType::Ground,   // Physical pin 25
    PinType::Gpio(7),  // Physical pin 26
    PinType::Gpio(0),  // Physical pin 27
    PinType::Gpio(1),  // Physical pin 28
    PinType::Gpio(5),  // Physical pin 29
    PinType::Ground,   // Physical pin 30
    PinType::Gpio(6),  // Physical pin 31
    PinType::Gpio(12), // Physical pin 32
    PinType::Gpio(13), // Physical pin 33
    PinType::Ground,   // Physical pin 34
    PinType::Gpio(19), // Physical pin 35
    PinType::Gpio(16), // Physical pin 36
    PinType::Gpio(26), // Physical pin 37
    PinType::Gpio(20), // Physical pin 38
    PinType::Ground,   // Physical pin 39
    PinType::Gpio(21), // Physical pin 40
];

const MAX_PINS_SHORT: usize = 26;
const MAX_PINS_LONG: usize = 40;

const SCK_PIN_ENV_VAR: &str = "SCK_PIN";
const SIN_PIN_ENV_VAR: &str = "SIN_PIN";
const SOUT_PIN_ENV_VAR: &str = "SOUT_PIN";
const SD_PIN_ENV_VAR: &str = "SD_PIN";

const MODE_ENV_VAR: &str = "MODE";

pub struct GameBoyLinkPins {
    sck: Line,
    sin: Line,
    sout: Line,
    sd: Line,
}

impl GameBoyLinkPins {
    fn from_env_vars() -> Result<Self, Box<dyn Error>> {
        let mut header_rev1 = HEADER;
        let (gpio_pins, max_pins) = match DeviceInfo::new()?.model() {
            Model::RaspberryPiBRev1 => {
                // The GPIO header on the earlier Pi models mostly overlaps with the
                // first 26 pins of the 40-pin header on the newer models. A few
                // pins are switched on the Pi B Rev 1.
                header_rev1[2] = PinType::Gpio(0);
                header_rev1[4] = PinType::Gpio(1);
                header_rev1[12] = PinType::Gpio(21);

                (&header_rev1[..MAX_PINS_SHORT], MAX_PINS_SHORT)
            }
            Model::RaspberryPiA | Model::RaspberryPiBRev2 => {
                (&HEADER[..MAX_PINS_SHORT], MAX_PINS_SHORT)
            }
            Model::RaspberryPiAPlus
            | Model::RaspberryPiBPlus
            | Model::RaspberryPi2B
            | Model::RaspberryPi3APlus
            | Model::RaspberryPi3B
            | Model::RaspberryPi3BPlus
            | Model::RaspberryPi4B
            | Model::RaspberryPiZero
            | Model::RaspberryPiZeroW => (&HEADER[..MAX_PINS_LONG], MAX_PINS_LONG),
            model => {
                return Err(<Box<dyn Error>>::from(format!(
                    "Error: No GPIO header information available for {}",
                    model
                )));
            }
        };

        let mut chip = Chip::new("/dev/gpiochip0")?;

        let mut pin_from_env_var = move |env_var: &str| -> Result<Line, Box<dyn Error>> {
            let pin_index = env::var(env_var)?.parse::<usize>()? - 1;
            if pin_index > 1 && pin_index < max_pins {
                let pin = gpio_pins[pin_index];
                if let PinType::Gpio(bcm_pin) = pin {
                    chip.get_line(bcm_pin as u32)
                        .map_err(<Box<dyn Error>>::from)
                } else {
                    return Err(<Box<dyn Error>>::from(format!(
                        "Error: {} is set to {} which is not a GPIO pin",
                        env_var, pin_index
                    )));
                }
            } else {
                return Err(<Box<dyn Error>>::from(format!(
                    "Error: {} is set to {} which is not a number in range [1, {}]",
                    env_var, pin_index, max_pins
                )));
            }
        };

        Ok(Self {
            sck: pin_from_env_var(SCK_PIN_ENV_VAR)?,
            sin: pin_from_env_var(SIN_PIN_ENV_VAR)?,
            sout: pin_from_env_var(SOUT_PIN_ENV_VAR)?,
            sd: pin_from_env_var(SD_PIN_ENV_VAR)?,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // TODO: Control the mode via HTTP
    let mode = env::var(MODE_ENV_VAR)?;

    match mode.as_str() {
        "printer" => {
            let pins = GameBoyLinkPins::from_env_vars()?;
            printer::main_loop(pins);
        }
        "pokemon_trade" => {
            // loop {
            //     // TODO: Figure out if this is necessary for peripherals
            //     tokio::time::sleep(*GAME_BOY_CLOCK_SPEED).await;
            // }
        }
        _ => {}
    };

    Ok(())
}
