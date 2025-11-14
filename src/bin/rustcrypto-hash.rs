#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::info;
use defmt_rtt as _;
use panic_probe as _;
use sha2::{Digest, Sha256};
use stm32wba::stm32wba55;

#[entry]
fn main() -> ! {
    // Start
    let msg = b"example message example message example message example message example message example message example message example message example message example message example message example message example message example message example message example message ";
    // Create a hasher and feed data
    let p = stm32wba55::Peripherals::take().unwrap();
    let gpio = &p.GPIOA;
    let clock = &p.RCC;

    // Enable HSI as a stable clock source
    clock.rcc_cr().modify(|_, w| w.hseon().set_bit());
    while clock.rcc_cr().read().hserdy().bit_is_clear() {
        cortex_m::asm::nop();
    }

    // Enable RNG clock. Select the source clock. Select the AHB clock
    clock.rcc_ccipr2().write(|w| w.rngsel().b_0x2());

    clock.rcc_ahb2enr().modify(|_, w| w.gpioaen().set_bit());

    // set pin to putput mode
    gpio.gpioa_moder()
        .modify(|_, w| unsafe { w.mode12().bits(0b01) }); // PA15 as output
    // set output type to push-pull
    gpio.gpioa_otyper().modify(|_, w| w.ot12().clear_bit());
    // set speed to low
    gpio.gpioa_ospeedr()
        .modify(|_, w| unsafe { w.ospeed12().bits(0b00) });
    // no pull-up/pull-down
    gpio.gpioa_pupdr()
        .modify(|_, w| unsafe { w.pupd12().bits(0b00) });

    // set initial state to low
    gpio.gpioa_bsrr().write(|w| w.br12().set_bit());

    loop {
        gpio.gpioa_bsrr().write(|w| w.bs12().set_bit()); // set high
        let mut hasher = Sha256::new();
        hasher.update(msg);
        let digest = hasher.finalize();
        gpio.gpioa_bsrr().write(|w| w.br12().set_bit()); // set low

        info!("SHA256(AAA) = {:x}", digest.as_slice());

        for _ in 0..200_000 {
            cortex_m::asm::nop();
        }
    }

    // loop {
    //     cortex_m::asm::nop();
    // }
}
