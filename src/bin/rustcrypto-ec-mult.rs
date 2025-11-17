#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::info;
use defmt_rtt as _;
use p256::{
    AffinePoint, EncodedPoint, ProjectivePoint, Scalar,
    elliptic_curve::{PrimeField, sec1::FromEncodedPoint, sec1::ToEncodedPoint},
};
use panic_probe as _;
use stm32wba::stm32wba55;
#[entry]
fn main() -> ! {
    // Start
    info!("Start");

    // scalar = 2
    let k_bytes: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 4,
    ];
    let k = Scalar::from_repr(k_bytes.into()).unwrap();

    // k=3
    // 5ECBE4D1A6330A44C8F7EF951D4BF165E6C6B721EFADA985FB41661BC6E7FD6C
    let pub_x: [u8; 32] = [
        0x5E, 0xCB, 0xE4, 0xD1, 0xA6, 0x33, 0x0A, 0x44, 0xC8, 0xF7, 0xEF, 0x95, 0x1D, 0x4B, 0xF1,
        0x65, 0xE6, 0xC6, 0xB7, 0x21, 0xEF, 0xAD, 0xA9, 0x85, 0xFB, 0x41, 0x66, 0x1B, 0xC6, 0xE7,
        0xFD, 0x6C,
    ];
    // k=3
    // 8734640C4998FF7E374B06CE1A64A2ECD82AB036384FB83D9A79B127A27D5032
    let pub_y: [u8; 32] = [
        0x87, 0x34, 0x64, 0x0C, 0x49, 0x98, 0xFF, 0x7E, 0x37, 0x4B, 0x06, 0xCE, 0x1A, 0x64, 0xA2,
        0xEC, 0xD8, 0x2A, 0xB0, 0x36, 0x38, 0x4F, 0xB8, 0x3D, 0x9A, 0x79, 0xB1, 0x27, 0xA2, 0x7D,
        0x50, 0x32,
    ];

    let encoded = EncodedPoint::from_affine_coordinates(&pub_x.into(), &pub_y.into(), false);

    let p_affine = AffinePoint::from_encoded_point(&encoded).unwrap();

    let pp: ProjectivePoint = ProjectivePoint::from(p_affine);
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

    // perform Q = k*P
    loop {
        gpio.gpioa_bsrr().write(|w| w.bs12().set_bit()); // set high

        let q = pp * k;
        let encoded = q.to_encoded_point(false);
        let x = encoded.x().unwrap();
        let y = encoded.y().unwrap();
        gpio.gpioa_bsrr().write(|w| w.br12().set_bit()); // set low

        info!("EC Mult k·P");
        info!("X = {:02x}", x.as_slice());
        info!("Y = {:02x}", y.as_slice());

        for _ in 0..200_000 {
            cortex_m::asm::nop();
        }
    }

    // loop {
    //     cortex_m::asm::nop();
    // }
}
