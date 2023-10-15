#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use fugit::HertzU32;
use micromath::F32Ext;
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;

pub mod sbus;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    uart::{DataBits, Parity, StopBits, UartConfig},
    watchdog::Watchdog,
};
use bsp::{entry, hal};

// use micromath::F32;

fn try_to_get_data<D, P>(
    uart: &hal::uart::UartPeripheral<hal::uart::Enabled, D, P>,
    recv: &mut sbus::Receiver,
) -> Option<sbus::Data>
where
    D: hal::uart::UartDevice,
    P: hal::uart::ValidUartPinout<D>,
{
    match uart.read_raw(recv.free_buf()) {
        Ok(n) => recv.read_bytes(n),
        Err(err) => match err {
            nb::Error::WouldBlock => return None,
            nb::Error::Other(err) => {
                let msg = match err.err_type {
                    hal::uart::ReadErrorType::Overrun => "overrun",
                    hal::uart::ReadErrorType::Break => "break",
                    hal::uart::ReadErrorType::Parity => "parity",
                    hal::uart::ReadErrorType::Framing => "framing",
                };
                defmt::error!("error reading thing: {}", msg);
                return None;
            }
        },
    };
    recv.get_data()
}

const TIME_THRESHOLD: fugit::Duration<u64, 1, 1_000_000> =
    fugit::Duration::<u64, 1, 1_000_000>::millis(1_000);

#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format, Default)]
pub enum Button {
    Pressed,
    #[default]
    Released,
}

impl Button {
    pub fn from_channel(val: u16) -> Self {
        match val {
            x if x > 1000 => Self::Pressed,
            _ => Self::Released,
        }
    }
}

#[derive(Clone, Copy, Debug, defmt::Format, Default)]
pub struct Axis {
    /// From -1.0 to 1.0
    val: f32,
}

impl Axis {
    pub fn new(val: f32) -> Self {
        Self { val }
    }

    pub fn from_channel(val: u16) -> Self {
        let val = (val as f32 - 1000.0) / 800.0;
        let val = match val {
            x if x.abs() < 0.02 => 0.0,
            x => x.clamp(-1.0, 1.0),
        };
        Self { val }
    }
}

impl PartialEq for Axis {
    fn eq(&self, other: &Axis) -> bool {
        (self.val - other.val).abs() <= f32::EPSILON
    }
}

impl Eq for Axis {}

#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format, Default)]
pub struct Stick {
    x: Axis,
    y: Axis,
}

impl Stick {
    pub fn new(x: Axis, y: Axis) -> Self {
        Self { x, y }
    }

    pub fn from_channels(x: u16, y: u16) -> Self {
        Self {
            x: Axis::from_channel(x),
            y: Axis::from_channel(y),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format, Default)]
pub enum ThreeWay {
    Up,
    #[default]
    Mid,
    Down,
}

impl ThreeWay {
    pub fn from_channel(val: u16) -> Self {
        match val {
            1800 => Self::Up,
            1000 => Self::Mid,
            200 => Self::Down,
            _ => Self::Mid,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format, Default)]
pub struct RadioLinkController {
    left_thumb: Stick,
    left_shoulder: Axis,
    left_trigger: ThreeWay,
    right_thumb: Stick,
    right_shoulder: Button,
    right_trigger: ThreeWay,
}

impl RadioLinkController {
    pub fn from_channels(chs: &[sbus::Chan; 16]) -> Self {
        Self {
            right_thumb: Stick::from_channels(chs[0].get(), chs[1].get()),
            left_thumb: Stick::from_channels(chs[3].get(), chs[2].get()),
            right_trigger: ThreeWay::from_channel(chs[4].get()),
            right_shoulder: Button::from_channel(chs[5].get()),
            left_trigger: ThreeWay::from_channel(chs[6].get()),
            left_shoulder: Axis::from_channel(chs[7].get()),
        }
    }
}

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    // let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

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

    let mut timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // (tx, rx)
    let uart_pins = (pins.gpio4.into_function(), pins.gpio5.into_function());

    let uart = hal::uart::UartPeripheral::new(pac.UART1, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(
                HertzU32::from_raw(100_000),
                DataBits::Eight,
                Some(Parity::Even),
                StopBits::Two,
            ),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    let mut last_data = None;

    let mut sbus_recv = sbus::Receiver::new();
    loop {
        let counter = timer.get_counter();
        let since_last_data = match last_data {
            None => None,
            Some(last) => counter.checked_duration_since(last),
        };
        // if let Some(last) = since_last_data {
        //     if last > TIME_THRESHOLD {
        //         info!("since last message = {}", last);
        //     }
        // }
        if let Some(data) = try_to_get_data(&uart, &mut sbus_recv) {
            last_data = Some(counter);
            // info!("sbus data = {}", data);
            let controller = RadioLinkController::from_channels(data.channels());
            info!("controller = {}", controller);
        }
    }
}
