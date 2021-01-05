use arraydeque::{ArrayDeque, Wrapping};
use core::ops::{Deref, DerefMut};
use embedded_hal::digital::{InputPin, OutputPin};

use crate::BUF_LEN;

#[derive(Debug)]
pub struct Controller<E, SCK, SIN, SOUT>
where
    SCK: OutputPin<Error = E>,
    SIN: InputPin<Error = E>,
    SOUT: OutputPin<Error = E>,
{
    sck: SCK,
    sin: SIN,
    sout: SOUT,
    high: bool,
    gb_sin: u8,
    gb_sout: u8,
    gb_bit: u8,
    send_buf: ArrayDeque<[u8; BUF_LEN], Wrapping>,
}

impl<E, SCK, SIN, SOUT> Controller<E, SCK, SIN, SOUT>
where
    SCK: OutputPin<Error = E>,
    SIN: InputPin<Error = E>,
    SOUT: OutputPin<Error = E>,
{
    pub fn new(sck: SCK, sin: SIN, sout: SOUT) -> Self {
        Self {
            sck,
            sin,
            sout,
            high: true,
            gb_sin: 0,
            gb_sout: 0,
            gb_bit: 0,
            send_buf: ArrayDeque::new(),
        }
    }

    pub fn send(&mut self) -> Result<Option<u8>, E> {
        if self.gb_bit == 0 {
            if self.send_buf.is_empty() {
                self.gb_sout = 0;
                return Ok(None);
            } else {
                if let Some(bit) = self.send_buf.pop_front() {
                    self.gb_sout = bit;
                } else {
                    return Ok(None);
                }
            }
        }

        let mut result = None;

        if self.high {
            if self.gb_sout & 0x80 > 0 {
                self.sout.try_set_high()?;
            } else {
                self.sout.try_set_low()?;
            }

            self.sck.try_set_low()?;
        } else {
            self.gb_sin |= if self.sin.try_is_high()? { 1 } else { 0 };
            self.gb_bit += 1;

            if self.gb_bit < 8 {
                self.gb_sout <<= 1;
                self.gb_sin <<= 1;
            } else {
                result = Some(self.gb_sin);

                self.gb_bit = 0;
                self.gb_sin = 0;
            }

            self.sck.try_set_high()?;
        }

        self.high = !self.high;

        Ok(result)
    }
}

impl<E, SCK, SIN, SOUT> Deref for Controller<E, SCK, SIN, SOUT>
where
    SCK: OutputPin<Error = E>,
    SIN: InputPin<Error = E>,
    SOUT: OutputPin<Error = E>,
{
    type Target = ArrayDeque<[u8; BUF_LEN], Wrapping>;

    fn deref(&self) -> &Self::Target {
        &self.send_buf
    }
}

impl<E, SCK, SIN, SOUT> DerefMut for Controller<E, SCK, SIN, SOUT>
where
    SCK: OutputPin<Error = E>,
    SIN: InputPin<Error = E>,
    SOUT: OutputPin<Error = E>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.send_buf
    }
}
