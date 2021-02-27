use arraydeque::{ArrayDeque, Wrapping};
use core::ops::{Deref, DerefMut};
use embedded_hal::digital::{InputPin, OutputPin};

use crate::BUF_LEN;

#[derive(Debug)]
pub struct Peripheral<E, SCK, SIN, SOUT>
where
    SCK: InputPin<Error = E>,
    SIN: OutputPin<Error = E>,
    SOUT: InputPin<Error = E>,
{
    sck: SCK,
    sin: SIN,
    sout: SOUT,
    gb_sin: u8,
    gb_sout: u8,
    gb_bit: u8,
    recv_buf: ArrayDeque<[u8; BUF_LEN], Wrapping>,
}

impl<E, SCK, SIN, SOUT> Peripheral<E, SCK, SIN, SOUT>
where
    SCK: InputPin<Error = E>,
    SIN: OutputPin<Error = E>,
    SOUT: InputPin<Error = E>,
{
    pub fn new(sck: SCK, sin: SIN, sout: SOUT) -> Self {
        Self {
            sck,
            sin,
            sout,
            gb_sin: 0,
            gb_sout: 0,
            gb_bit: 0,
            recv_buf: ArrayDeque::new(),
        }
    }

    pub fn recv(&mut self) -> Result<Option<u8>, E> {
        if self.gb_bit == 0 {
            if self.recv_buf.is_empty() {
                self.gb_sin = 0;
            } else {
                if let Some(bit) = self.recv_buf.pop_front() {
                    self.gb_sin = bit;
                }
            }
        }

        if self.sck.try_is_high()? {
            if (self.gb_sin & 0x80) > 0 {
                self.sin.try_set_high()?;
            } else {
                self.sin.try_set_low()?;
            }

            return Ok(None);
        }

        // println!("{}: {:08b} | {}: {:08b}", self.gb_bit, self.gb_sout, if self.sout.try_is_high()? { 1 } else { 0 }, self.gb_sout | if self.sout.try_is_high()? { 1 } else { 0 });

        self.gb_sout |= if self.sout.try_is_high()? { 1 } else { 0 };
        self.gb_bit += 1;

        if self.gb_bit < 8 {
            self.gb_sin <<= 1;
            self.gb_sout <<= 1;

            return Ok(None);
        }

        let result = Ok(Some(self.gb_sout));

        self.gb_bit = 0;
        self.gb_sout = 0;

        result
    }

    pub fn reset(&mut self) {
        self.gb_sin = 0;
        self.gb_sout = 0;
        self.gb_bit = 0;
        self.recv_buf.drain(..);
    }
}

impl<E, SCK, SIN, SOUT> Deref for Peripheral<E, SCK, SIN, SOUT>
where
    SCK: InputPin<Error = E>,
    SIN: OutputPin<Error = E>,
    SOUT: InputPin<Error = E>,
{
    type Target = ArrayDeque<[u8; BUF_LEN], Wrapping>;

    fn deref(&self) -> &Self::Target {
        &self.recv_buf
    }
}

impl<E, SCK, SIN, SOUT> DerefMut for Peripheral<E, SCK, SIN, SOUT>
where
    SCK: InputPin<Error = E>,
    SIN: OutputPin<Error = E>,
    SOUT: InputPin<Error = E>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.recv_buf
    }
}
