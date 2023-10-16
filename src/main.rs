#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embedded_hal::PwmPin;
use fugit::HertzU32;
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;

pub mod motion;
pub mod rc;
pub mod sbus;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    uart::{DataBits, Parity, StopBits, UartConfig},
    watchdog::Watchdog,
};
use bsp::{entry, hal};

use crate::{
    motion::{Car, Motor},
    rc::RadioLinkController,
};

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

    // Init PWMs
    let mut pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);
    pwm_slices.pwm0.set_ph_correct();
    pwm_slices.pwm0.enable();
    pwm_slices.pwm1.set_ph_correct();
    pwm_slices.pwm1.enable();

    let chan_lf = {
        let channel = &mut pwm_slices.pwm0.channel_a;
        channel.output_to(pins.gpio16);
        channel.set_duty(0);
        channel
    };

    let chan_lr = {
        let channel = &mut pwm_slices.pwm0.channel_b;
        channel.output_to(pins.gpio17);
        channel.set_duty(0);
        channel
    };
    let chan_rf = {
        let channel = &mut pwm_slices.pwm1.channel_a;
        channel.output_to(pins.gpio18);
        channel.set_duty(0);
        channel
    };

    let chan_rr = {
        let channel = &mut pwm_slices.pwm1.channel_b;
        channel.output_to(pins.gpio19);
        channel.set_duty(0);
        channel
    };

    let mut last_data = None;

    let mut sbus_recv = sbus::Receiver::new();
    let mut car = Car {
        left: Motor::new(),
        right: Motor::new(),
        // right: Motor::new(pins.gpio17, pins.gpio19),
    };
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
            let controller = RadioLinkController::from_channels(&data.channels);
            // info!("controller = {}", controller);
            car.update(controller.left_thumb.y.val, controller.right_thumb.y.val);
            car.left.drive(chan_lf, chan_lr);
            car.right.drive(chan_rf, chan_rr);
        }
    }
}
