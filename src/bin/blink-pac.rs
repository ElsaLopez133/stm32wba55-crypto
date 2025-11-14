#![no_std]
#![no_main]

// https://www.st.com/resource/en/user_manual/dm01047885.pdf Section 7.10

use cortex_m::asm;
use cortex_m_rt::entry;
use defmt::*;
use stm32wba::stm32wba55;
use {defmt_rtt as _, panic_probe as _};

pub struct GpioPA9<'a> {
    gpioa: &'a stm32wba55::GPIOA,
}

impl<'a> GpioPA9<'a> {
    /// Initialize PA9 as push-pull output
    pub fn new(p: &'a stm32wba55::Peripherals) -> Self {
        // Enable GPIOA clock
        p.RCC.rcc_ahb2enr().modify(|_, w| w.gpioaen().set_bit());

        // Set PA9 to output mode
        p.GPIOA
            .gpioa_moder()
            .modify(|_, w| unsafe { w.mode9().bits(0b01) });

        // Set output type to push-pull
        p.GPIOA.gpioa_otyper().modify(|_, w| w.ot9().clear_bit());

        // Set speed to low
        p.GPIOA
            .gpioa_ospeedr()
            .modify(|_, w| unsafe { w.ospeed9().bits(0b00) });

        // No pull-up/pull-down
        p.GPIOA
            .gpioa_pupdr()
            .modify(|_, w| unsafe { w.pupd9().bits(0b00) });

        // Initialize to a known state (optional)
        let gpio = GpioPA9 { gpioa: &p.GPIOA };

        // Set initial state to low (optional)
        gpio.set_low();

        gpio
    }

    /// Set PA9 high
    pub fn set_high(&self) {
        self.gpioa.gpioa_bsrr().write(|w| w.bs9().set_bit());
    }

    /// Set PA9 low
    pub fn set_low(&self) {
        self.gpioa.gpioa_bsrr().write(|w| w.br9().set_bit());
    }
}

#[entry]
unsafe fn main() -> ! {
    // let p = embassy_stm32::init(Default::default());
    // let mut led = Output::new(p.PA9, Level::High, Speed::Low);

    let p = stm32wba55::Peripherals::take().unwrap();
    let led = GpioPA9::new(&p);
    // let mut led = Output::new(p.PA12, Level::High, Speed::Low);

    loop {
        info!("high");
        led.set_high();
        delay();

        info!("low");
        led.set_low();
        delay();
    }
}

fn delay() {
    for _ in 0..100_000 {
        cortex_m::asm::nop();
    }
}
