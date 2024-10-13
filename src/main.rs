// Blinks the LED on the LoRa-E5_STM32WLE5JC_Module.

#![no_std]
#![no_main]

use defmt_rtt as _; // global logger
use panic_probe as _; // panic handler
use stm32wlxx_hal::{
    self as hal,
    cortex_m::{self, delay::Delay},
    gpio::{pins, Output, PinState, PortC},
    info::{self, Package, Uid, Uid64},
    pac,
    util::new_delay,
};

#[hal::cortex_m_rt::entry]
fn main() -> ! {
    let mut dp: pac::Peripherals = defmt::unwrap!(pac::Peripherals::take());
    let cp: pac::CorePeripherals = defmt::unwrap!(pac::CorePeripherals::take());

    let gpioc: PortC = PortC::split(dp.GPIOC, &mut dp.RCC);
    let mut led1: Output<pins::C0> =
        cortex_m::interrupt::free(|cs| {
            Output::default(gpioc.c0, cs)            
        });

    let mut delay: Delay = new_delay(cp.SYST, &dp.RCC);

    defmt::println!("Flash size: {} KiB", info::flash_size_kibibyte());
    defmt::println!("Package: {:?}", Package::from_device());
    defmt::println!("UID64: {}", Uid64::from_device());
    defmt::println!("UID: {}", Uid::from_device());

    defmt::info!("Starting blinky");

    loop {
        for &level in &[PinState::High, PinState::Low] {
            led1.set_level(level);
            delay.delay_ms(1000);
        }
    }
}