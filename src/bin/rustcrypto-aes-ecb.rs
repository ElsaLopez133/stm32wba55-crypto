#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::info;
use defmt_rtt as _;
use panic_probe as _;
use stm32wba::stm32wba55;

use aes::Aes128;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockEncrypt, KeyInit};

#[entry]
fn main() -> ! {
    // 128-bit AES key
    let key = GenericArray::from([
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff,
    ]);

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

    let cipher = Aes128::new(&key);

    loop {
        // 16-bytes plaintext block
        let mut block = GenericArray::from([
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ]);

        gpio.gpioa_bsrr().write(|w| w.bs12().set_bit()); // set high
        cipher.encrypt_block(&mut block);
        gpio.gpioa_bsrr().write(|w| w.br12().set_bit()); // set low

        info!("AES-128-ECB(plaintext) = {:02x}", block.as_slice());

        for _ in 0..200_000 {
            cortex_m::asm::nop();
        }
    }

    // loop {
    //     cortex_m::asm::nop();
    // }
}
