use crate::GameBoyLinkPins;
use bitflags::bitflags;
use embedded_hal::digital::{InputPin, OutputPin, PinState};
use futures::StreamExt;
use game_boy_link_driver::Peripheral;
use gpio_cdev::{EventRequestFlags, LineRequestFlags};
use linux_embedded_hal::CdevPin;
use num::FromPrimitive;
use num_derive::{FromPrimitive, ToPrimitive};
use std::{
    error::Error,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

const PRINTER_MAGIC_0: u8 = 0x88;
const PRINTER_MAGIC_1: u8 = 0x33;

pub enum PrinterState {
    Magic0,
    Magic1,
    Cmd,
    Compression {
        cmd: PrintCommand,
    },
    LenLow {
        cmd: PrintCommand,
        compression: PrintCompression,
    },
    LenHigh {
        cmd: PrintCommand,
        compression: PrintCompression,
        len_low: u8,
    },
    Data {
        cmd: PrintCommand,
        compression: PrintCompression,
        len: u16,
        index: u16,
        payload: Vec<u8>,
    },
    CheckSum0 {
        cmd: PrintCommand,
        compression: PrintCompression,
        payload: Vec<u8>,
    },
    CheckSum1 {
        cmd: PrintCommand,
        compression: PrintCompression,
        payload: Vec<u8>,
        checksum0: u8,
    },
    Keepalive {
        cmd: PrintCommand,
        compression: PrintCompression,
        payload: Vec<u8>,
        checksum: u16,
        checksum_expected: u16,
        checksum_error: bool,
    },
    Status {
        cmd: PrintCommand,
        compression: PrintCompression,
        payload: Vec<u8>,
        checksum: u16,
        checksum_expected: u16,
        checksum_error: bool,
    },
    Done {
        cmd: PrintCommand,
        compression: PrintCompression,
        payload: Vec<u8>,
        checksum: u16,
        checksum_expected: u16,
        checksum_error: bool,
    },
}

impl PrinterState {
    pub fn transition(self, input: u8) -> Result<Self, Box<dyn Error>> {
        match self {
            Self::Magic0 => {
                if input == PRINTER_MAGIC_0 {
                    println!("Got magic byte 0!");
                    Ok(Self::Magic1)
                } else {
                    Ok(Self::Magic0)
                    // Err(<Box<dyn Error>>::from(format!(
                    //     "Unknown first magic byte {:#004x}. Expected 0x88.",
                    //     input
                    // )))
                }
            }
            Self::Magic1 => {
                if input == PRINTER_MAGIC_1 {
                    println!("Got magic byte 1!");
                    Ok(Self::Cmd)
                } else {
                    // Err(<Box<dyn Error>>::from(format!(
                    //     "Unknown second magic byte {:#004x}. Expected 0x33.",
                    //     input
                    // )))
                    Ok(Self::Magic0)
                }
            }
            Self::Cmd => {
                if let Some(cmd) = PrintCommand::from_u8(input) {
                    Ok(Self::Compression { cmd })
                } else {
                    Err(<Box<dyn Error>>::from(format!(
                        "Unknown command: {:#004x}. Expected one of 0x01, 0x02, 0x04, or 0x0F.",
                        input
                    )))
                }
            }
            Self::Compression { cmd } => {
                if let Some(compression) = PrintCompression::from_u8(input) {
                    Ok(Self::LenLow { cmd, compression })
                } else {
                    Err(<Box<dyn Error>>::from(format!(
                        "Unknown compression type: {:#004x}. Expected 0x00 or 0x01.",
                        input
                    )))
                }
            }
            Self::LenLow { cmd, compression } => Ok(Self::LenHigh {
                cmd,
                len_low: input,
                compression,
            }),
            Self::LenHigh {
                cmd,
                compression,
                len_low,
            } => {
                let len = u16::from_le_bytes([len_low, input]);
                let payload = vec![0; len as usize];
                if len > 0 {
                    Ok(Self::Data {
                        cmd,
                        compression,
                        len,
                        index: len,
                        payload,
                    })
                } else {
                    Ok(Self::CheckSum0 {
                        cmd,
                        compression,
                        payload,
                    })
                }
            }
            Self::Data {
                cmd,
                compression,
                len,
                index,
                mut payload,
            } => {
                payload[(len - index) as usize] = input;
                let index = index - 1;
                if index > 0 {
                    Ok(Self::Data {
                        cmd,
                        compression,
                        len,
                        index,
                        payload,
                    })
                } else {
                    Ok(Self::CheckSum0 {
                        cmd,
                        compression,
                        payload,
                    })
                }
            }
            Self::CheckSum0 {
                cmd,
                compression,
                payload,
            } => Ok(Self::CheckSum1 {
                cmd,
                compression,
                payload,
                checksum0: input,
            }),
            Self::CheckSum1 {
                cmd,
                compression,
                payload,
                checksum0,
            } => {
                let checksum = payload.iter().fold(0_u16, |acc, byte| acc + *byte as u16);
                let checksum_expected = u16::from_le_bytes([checksum0, input]);
                Ok(Self::Keepalive {
                    cmd,
                    compression,
                    payload,
                    checksum,
                    checksum_expected,
                    checksum_error: checksum == checksum_expected,
                })
            }
            Self::Keepalive {
                cmd,
                compression,
                payload,
                checksum,
                checksum_expected,
                checksum_error,
            } => {
                if input == 0x0 {
                    Ok(Self::Status {
                        cmd,
                        compression,
                        payload,
                        checksum,
                        checksum_expected,
                        checksum_error,
                    })
                } else {
                    Err(<Box<dyn Error>>::from(format!(
                        "Unknown keepalive byte: {:#004x}. Expected 0x00.",
                        input
                    )))
                }
            }
            Self::Status {
                cmd,
                compression,
                payload,
                checksum,
                checksum_expected,
                checksum_error,
            } => {
                if input == 0x0 {
                    Ok(Self::Done {
                        cmd,
                        compression,
                        payload,
                        checksum,
                        checksum_expected,
                        checksum_error,
                    })
                } else {
                    Err(<Box<dyn Error>>::from(format!(
                        "Unknown status byte: {:#004x}. Expected 0x00.",
                        input
                    )))
                }
            }
            Self::Done { .. } => Err(<Box<dyn Error>>::from(
                "Printer packet is already in its final state",
            )),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone, ToPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum PrintCommand {
    Init = 0x01,
    Print = 0x02,
    Data = 0x04,
    Status = 0x0f,
}

#[derive(Debug, PartialEq, Copy, Clone, ToPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum PrintCompression {
    Uncompressed = 0x00,
    Compressed = 0x01,
}

#[derive(Debug, PartialEq, Copy, Clone, ToPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum PrinterAlive {
    Dead = 0x80,
    Alive = 0x81,
}

bitflags! {
    pub struct PrinterStatus: u8 {
        const OK =               0b00000000;
        const CHECKSUM_ERROR =   0b00000001;
        const PRINTER_BUSY =     0b00000010;
        const IMAGE_DATA_FULL =  0b00000100;
        const UNPROCESSED_DATA = 0b00001000;
        const PACKET_ERROR =     0b00010000;
        const PAPER_JAM =        0b00100000;
        const OTHER_ERROR =      0b01000000;
        const BATTERY_TOO_LOW =  0b10000000;
    }
}

pub fn decompress_data(bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut decompressed_data = vec![];

    let byte_iter = &mut bytes.iter();

    while let Some(run_byte) = byte_iter.next() {
        let run_byte = *run_byte as usize;
        let compressed_run = (run_byte >> 7) == 1;

        if compressed_run {
            let run_len = !(!run_byte | (1 << 7)) + 2;

            if let Some(repeated_byte) = byte_iter.next() {
                decompressed_data.extend(vec![*repeated_byte; run_len]);
            } else {
                return Err(<Box<dyn Error>>::from(
                    "Repeat byte not found in compressed run.",
                ));
            }
        } else {
            let run_len = run_byte + 1;
            let run_bytes = byte_iter.take(run_len);

            if run_bytes.len() == run_len {
                decompressed_data.extend(run_bytes);
            } else {
                return Err(<Box<dyn Error>>::from(
                    "Not enough bytes found in uncompressed run.",
                ));
            }
        }
    }

    Ok(decompressed_data)
}

pub async fn main_loop(pins: GameBoyLinkPins) -> Result<(), Box<dyn Error>> {
    let mut sck_events = pins.sck.async_events(
        LineRequestFlags::INPUT,
        EventRequestFlags::BOTH_EDGES,
        "game-boy-line-online",
    )?;

    // Emulated pin that is driven by events from the real SCK pin
    let mut sck = AtomicSck::default();

    let sin = CdevPin::new(pins.sin.request(
        LineRequestFlags::OUTPUT,
        0,
        "game-boy-line-online",
    )?)?;
    let sout = CdevPin::new(pins.sout.request(
        LineRequestFlags::INPUT,
        0,
        "game-boy-line-online",
    )?)?;

    let mut peripheral = Peripheral::new(sck.clone(), sin, sout);

    let mut printer_state = PrinterState::Magic0;

    let mut last_line_event_type = None;

    let mut now = Instant::now();

    loop {
        if let Some(event) = sck_events.next().await {
            // If some time passes, reset the peripheral
            let elapsed = now.elapsed();
            if elapsed.as_secs() > 1 {
                peripheral.reset();
                // println!("Reset the peripheral");
            }
            if elapsed.as_micros() > 200 {
                println!("Elapsed time: {}us", elapsed.as_micros());
            }
            // println!("Elapsed time: {}us", elapsed.as_micros());
            now = Instant::now();

            if let Ok(line_event) = event {
                // Sometimes there are duplicate events, debounce them
                let line_event_type = line_event.event_type();
                match &last_line_event_type {
                    Some(llet) if llet == &line_event_type => {
                        continue;
                    }
                    _ => {
                        last_line_event_type.replace(line_event_type);
                    }
                };

                // dbg!(&line_event.event_type());

                match line_event.event_type() {
                    gpio_cdev::EventType::FallingEdge => {
                        sck.try_set_low()?;
                    }
                    gpio_cdev::EventType::RisingEdge => {
                        sck.try_set_high()?;
                    }
                };

                if let Some(byte) = peripheral.recv()? {
                    // dbg!(&byte);
                    printer_state = printer_state.transition(byte)?;
                    match printer_state {
                        PrinterState::Keepalive { .. } => {
                            peripheral.push_back(PrinterAlive::Alive as u8);
                        }
                        PrinterState::Status { checksum_error, .. } => {
                            let status = if checksum_error {
                                PrinterStatus::CHECKSUM_ERROR
                            } else {
                                PrinterStatus::OK
                            };
                            peripheral.push_back(status.bits());
                        }
                        PrinterState::Done {
                            compression,
                            payload,
                            ..
                        } => {
                            printer_state = PrinterState::Magic0;

                            let mut payload = payload;

                            if let PrintCompression::Compressed = compression {
                                payload = decompress_data(&payload)?;
                            }

                            // TODO: save image
                            println!("{:X?}", payload);
                        }
                        _ => {}
                    };
                }
            }
        } else {
            break;
        }
    }

    Ok(())
}

#[derive(Debug, Default, Clone)]
struct AtomicSck {
    is_high: Rc<AtomicBool>,
}

impl OutputPin for AtomicSck {
    type Error = gpio_cdev::errors::Error;

    fn try_set_high(&mut self) -> Result<(), Self::Error> {
        self.is_high.store(true, Ordering::Release);
        Ok(())
    }

    fn try_set_low(&mut self) -> Result<(), Self::Error> {
        self.is_high.store(false, Ordering::Release);
        Ok(())
    }

    fn try_set_state(&mut self, state: PinState) -> Result<(), Self::Error> {
        match state {
            PinState::High => self.try_set_high(),
            PinState::Low => self.try_set_low(),
        }
    }
}

impl InputPin for AtomicSck {
    type Error = gpio_cdev::errors::Error;

    fn try_is_high(&self) -> Result<bool, Self::Error> {
        Ok(self.is_high.load(Ordering::Acquire))
    }

    fn try_is_low(&self) -> Result<bool, Self::Error> {
        Ok(!self.is_high.load(Ordering::Acquire))
    }
}
