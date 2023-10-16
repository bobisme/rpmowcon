use core::marker::PhantomData;

use embedded_hal::PwmPin;

use bsp::hal;
use micromath::F32Ext;
use rp_pico as bsp;

#[derive(Clone, Default, Debug)]
pub struct Motor<FChan, RChan> {
    pub throttle: f32,
    _mark: PhantomData<(FChan, RChan)>,
}

impl<FChan, RChan> Motor<FChan, RChan>
where
    FChan: PwmPin<Duty = u16>,
    RChan: PwmPin<Duty = u16>,
{
    pub fn new() -> Self {
        Self {
            throttle: 0.0,
            _mark: PhantomData,
        }
    }

    pub fn duty(&self) -> u16 {
        // ((self.throttle + 1.0) / 2.0 * u16::MAX as f32) as u16
        (self.throttle.abs() * u16::MAX as f32) as u16
    }

    pub fn drive(&self, forward_ch: &mut FChan, reverse_ch: &mut RChan) {
        match self.throttle {
            t if t >= 0.0 => {
                forward_ch.set_duty(self.duty());
                reverse_ch.set_duty(0);
            }
            _ => {
                forward_ch.set_duty(0);
                reverse_ch.set_duty(self.duty());
            }
        };
    }
}

#[derive(Clone, Debug)]
pub struct Car<LFChan, LRChan, RFChan, RRChan> {
    pub left: Motor<LFChan, LRChan>,
    pub right: Motor<RFChan, RRChan>,
}

impl<LFChan, LRChan, RFChan, RRChan> Car<LFChan, LRChan, RFChan, RRChan> {
    pub fn update(&mut self, left_throttle: f32, right_throttle: f32) {
        self.left.throttle = left_throttle;
        self.right.throttle = right_throttle;
    }
}
