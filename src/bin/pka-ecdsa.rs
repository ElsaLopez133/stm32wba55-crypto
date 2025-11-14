#![no_std]
#![no_main]

// Reference Manual: file:///C:/Users/elopezpe/OneDrive/Documentos/PhD/micro/stm32eba55cg/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf
use core::{
    mem::size_of,
    ptr::{read_volatile, write_volatile},
};
use cortex_m::asm;
use cortex_m_rt::entry;
use defmt::info;
use stm32wba::stm32wba55::{self};
use {defmt_rtt as _, panic_probe as _};

const BASE: usize = 0x520C_2000;
const PKA_RAM_OFFSET: usize = 0x400;
const RAM_BASE: usize = BASE + PKA_RAM_OFFSET;
const RAM_NUM_DW: usize = 667;
const SIGN: u8 = 0x24;
const VERIFY: u8 = 0x26;
const OPERAND_LENGTH: u32 = 8 * 32;
const PRIME_ORDER_SIZE: usize = 8;
const MODULUS_SIZE: usize = 8;
const WORD_SIZE: usize = (OPERAND_LENGTH as usize) / 32;

// ECDSA sign input addresses
const ECDSA_SIGN_N_LEN: usize = BASE + 0x400;
const ECDSA_SIGN_P_LEN: usize = BASE + 0x408;
const ECDSA_SIGN_A_SIGN: usize = BASE + 0x410;
const ECDSA_SIGN_A: usize = BASE + 0x418;
const ECDSA_SIGN_B: usize = BASE + 0x520;
const ECDSA_SIGN_P: usize = BASE + 0x1088;
const ECDSA_SIGN_K: usize = BASE + 0x12A0;
const ECDSA_SIGN_X: usize = BASE + 0x578;
const ECDSA_SIGN_Y: usize = BASE + 0x470;
const ECDSA_SIGN_Z: usize = BASE + 0xFE8;
const ECDSA_SIGN_D: usize = BASE + 0xF28;
const ECDSA_SIGN_N: usize = BASE + 0xF88;

// ECDSA sign output addresses
const ECDSA_SIGN_OUT_R: usize = BASE + 0x730;
const ECDSA_SIGN_OUT_S: usize = BASE + 0x788;
const ECDSA_SIGN_OUT_RESULT: usize = BASE + 0xFE0;

// ECDSA verify input addresses
const ECDSA_VERIFY_N_LEN: usize = BASE + 0x408;
const ECDSA_VERIFY_P_LEN: usize = BASE + 0x4C8;
const ECDSA_VERIFY_A_SIGN: usize = BASE + 0x468;
const ECDSA_VERIFY_A: usize = BASE + 0x470;
const ECDSA_VERIFY_B: usize = BASE + 0x520;
const ECDSA_VERIFY_P: usize = BASE + 0x4D0;
const ECDSA_VERIFY_X: usize = BASE + 0x678;
const ECDSA_VERIFY_Y: usize = BASE + 0x6D0;
const ECDSA_VERIFY_XQ: usize = BASE + 0x12F8;
const ECDSA_VERIFY_YQ: usize = BASE + 0x1350;
const ECDSA_VERIFY_R: usize = BASE + 0x10E0;
const ECDSA_VERIFY_S: usize = BASE + 0xC68;
const ECDSA_VERIFY_Z: usize = BASE + 0x13A8;
const ECDSA_VERIFY_N: usize = BASE + 0x1088;

// ECDSA verify output addresses
const ECDSA_VERIFY_OUT: usize = BASE + 0x5D0;
const ECDSA_VERIFY_SIGN_OUT_R: usize = BASE + 0x578;

const PRIV_KEY: [u32; 8] = [
    0xC477F9F6, 0x5C22CCE2, 0x0657FAA5, 0xB2D1D812, 0x2336F851, 0xA508A1ED, 0x04E479C3, 0x4985BF96,
];

const CURVE_PT_X: [u32; 8] = [
    0xB7E08AFD, 0xFE94BAD3, 0xF1DC8C73, 0x4798BA1C, 0x62B3A0AD, 0x1E9EA2A3, 0x8201CD08, 0x89BC7A19,
];
const CURVE_PT_Y: [u32; 8] = [
    0x3603F747, 0x959DBF7A, 0x4BB226E4, 0x19287290, 0x63ADC7AE, 0x43529E61, 0xB563BBC6, 0x06CC5E09,
];

const A_SIGN: u32 = 0x1;

const A: [u32; 8] = [
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000003,
];
const P: [u32; 8] = [
    0xffffffff, 0x00000001, 0x00000000, 0x00000000, 0x00000000, 0xffffffff, 0xffffffff, 0xffffffff,
];

const B: [u32; 8] = [
    0x5ac635d8, 0xaa3a93e7, 0xb3ebbd55, 0x769886bc, 0x651d06b0, 0xcc53b0f6, 0x3bce3c3e, 0x27d2604b,
];

const BASE_POINT_X: [u32; 8] = [
    0x6b17d1f2, 0xe12c4247, 0xf8bce6e5, 0x63a440f2, 0x77037d81, 0x2deb33a0, 0xf4a13945, 0xd898c296,
];

const BASE_POINT_Y: [u32; 8] = [
    0x4fe342e2, 0xfe1a7f9b, 0x8ee7eb4a, 0x7c0f9e16, 0x2bce3357, 0x6b315ece, 0xcbb64068, 0x37bf51f5,
];

const PRIME_ORDER: [u32; 8] = [
    0xffffffff, 0x00000000, 0xffffffff, 0xffffffff, 0xbce6faad, 0xa7179e84, 0xf3b9cac2, 0xfc632551,
];

const NONCE: [u32; 8] = [
    0x7A1A7E52, 0x797FC8CA, 0xAA435D2A, 0x4DACE391, 0x58504BF2, 0x04FBE19F, 0x14DBB427, 0xFAEE50AE,
];

const HASH: [u32; 8] = [
    0xA41A41A1, 0x2A799548, 0x211C410C, 0x65D8133A, 0xFDE34D28, 0xBDD542E4, 0xB680CF28, 0x99C8A8C4,
];

unsafe fn write_ram(offset: usize, buf: &[u32]) {
    debug_assert_eq!(offset % 4, 0);
    debug_assert!(offset + buf.len() * size_of::<u32>() < 0x520C_33FF);
    buf.iter()
        .rev()
        .enumerate()
        .for_each(|(idx, &dw)| write_volatile((offset + idx * size_of::<u32>()) as *mut u32, dw));
}

unsafe fn read_ram(offset: usize, buf: &mut [u32]) {
    debug_assert_eq!(offset % 4, 0);
    debug_assert!(offset + buf.len() * size_of::<u32>() < 0x520C_33FF);
    buf.iter_mut().rev().enumerate().for_each(|(idx, dw)| {
        *dw = read_volatile((offset + idx * size_of::<u32>()) as *const u32);
    });
}

unsafe fn zero_ram() {
    (0..RAM_NUM_DW)
        .into_iter()
        .for_each(|dw| unsafe { write_volatile((dw * 4 + RAM_BASE) as *mut u32, 0) });
}

#[entry]
unsafe fn main() -> ! {
    let p = stm32wba55::Peripherals::take().unwrap();
    let pka = &p.PKA;
    let clock = &p.RCC;
    let rng = &p.RNG;
    let gpio = &p.GPIOA;

    // Enable HSI as a stable clock source
    clock.rcc_cr().modify(|_, w| w.hseon().set_bit());
    while clock.rcc_cr().read().hserdy().bit_is_clear() {
        asm::nop();
    }

    // Enable RNG clock. Select the source clock. Select the AHB clock
    clock.rcc_ccipr2().write(|w| w.rngsel().b_0x2());
    clock.rcc_ahb2enr().modify(|_, w| {
        w.rngen().set_bit();
        w.gpioaen().set_bit()
    });
    while clock.rcc_ahb2enr().read().rngen().bit_is_clear() {
        asm::nop();
    }

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
    gpio.gpioa_bsrr().write(
        |w: &mut stm32wba::raw::W<stm32wba55::gpioa::gpioa_bsrr::GPIOA_BSRRrs>| w.br12().set_bit(),
    );

    // Configure RNG
    // To configure, CONDRST bit is set to 1 in the same access and CONFIGLOCK remains at 0
    rng.rng_cr().write(|w| {
        w.rngen()
            .clear_bit()
            .condrst()
            .set_bit()
            .configlock()
            .clear_bit()
            .nistc()
            .clear_bit()
            .ced()
            .clear_bit()
    });

    // First clear CONDRST while keeping RNGEN disabled
    rng.rng_cr().modify(|_, w| w.condrst().clear_bit());

    // Then enable RNG in a separate step
    rng.rng_cr()
        .modify(|_, w| w.rngen().set_bit().ie().set_bit());

    while rng.rng_sr().read().drdy().bit_is_clear() {
        asm::nop();
    }
    info!("RNG enabled successfully");

    // Enable PKA peripheral clock via RCC_AHB2ENR register
    clock.rcc_ahb2enr().modify(|_, w| w.pkaen().set_bit());

    // Reset PKA before enabling (sometimes helps with initialization)
    pka.pka_cr().modify(|_, w| w.en().clear_bit());
    for _ in 0..10 {
        asm::nop();
    }

    // Enable PKA peripheral
    pka.pka_cr().write(
        |w| w.en().set_bit(), // .mode().bits(MODE)
    );

    // Wait for PKA to initialize
    while pka.pka_sr().read().initok().bit_is_clear() {
        asm::nop();
    }
    info!("PKA initialized successfully!");

    // Clear any previous error flags
    pka.pka_clrfr().write(|w| {
        w.addrerrfc()
            .set_bit()
            .ramerrfc()
            .set_bit()
            .procendfc()
            .set_bit()
    });

    // Write the values - using 32-bit words
    zero_ram();
    write_ram(ECDSA_SIGN_N_LEN, &[OPERAND_LENGTH]);
    write_ram(ECDSA_SIGN_P_LEN, &[OPERAND_LENGTH]);
    write_ram(ECDSA_SIGN_A_SIGN, &[A_SIGN]);
    write_ram(ECDSA_SIGN_A, &A);
    write_ram(ECDSA_SIGN_B, &B);
    write_ram(ECDSA_SIGN_P, &P);
    write_ram(ECDSA_SIGN_K, &NONCE);
    write_ram(ECDSA_SIGN_X, &BASE_POINT_X);
    write_ram(ECDSA_SIGN_Y, &BASE_POINT_Y);
    write_ram(ECDSA_SIGN_Z, &HASH);
    write_ram(ECDSA_SIGN_D, &PRIV_KEY);
    write_ram(ECDSA_SIGN_N, &PRIME_ORDER);

    // // Check the values
    // let mut buf = [0u32; WORD_SIZE];
    // read_ram(ECDSA_SIGN_A, &mut buf);
    // info!("ECDSA_SIGN_A: {:#X}", buf);
    // read_ram(ECDSA_SIGN_P, &mut buf);
    // info!("ECDSA_SIGN_P: {:#X}", buf);
    // read_ram(ECDSA_SIGN_K, &mut buf);
    // info!("ECDSA_SIGN_K: {:#X}", buf);
    // read_ram(ECDSA_SIGN_X, &mut buf);
    // info!("ECDSA_SIGN_X: {:#X}", buf);
    // read_ram(ECDSA_SIGN_Y, &mut buf);
    // info!("ECDSA_SIGN_Y: {:#X}", buf);
    // read_ram(ECDSA_SIGN_Z, &mut buf);
    // info!("ECDSA_SIGN_Z: {:#X}", buf);
    // read_ram(ECDSA_SIGN_D, &mut buf);
    // info!("ECDSA_SIGN_D: {:#X}", buf);
    // read_ram(ECDSA_SIGN_N, &mut buf);
    // info!("ECDSA_SIGN_N: {:#X}", buf);

    // Configure PKA operation mode and start
    info!("Starting SIGN operation...");
    gpio.gpioa_bsrr().write(|w| w.bs12().set_bit()); // set high

    pka.pka_cr().modify(
        |_, w| w.mode().bits(SIGN).start().set_bit(), // Start the operation
    );

    // Wait for processing to complete - PROCENDF is 1 when done
    info!("Waiting for operation to complete...");
    while pka.pka_sr().read().procendf().bit_is_clear() {
        asm::nop();
    }

    gpio.gpioa_bsrr().write(|w| w.br12().set_bit());
    info!("Operation complete!");

    // Read the result
    let mut result = [0u32; 1];
    let mut sign_out_r = [0u32; MODULUS_SIZE];
    let mut sign_out_s = [0u32; MODULUS_SIZE];
    read_ram(ECDSA_SIGN_OUT_RESULT, &mut result);
    if result[0] == 0xD60D {
        info!("No errors : {:#X}", result[0]);
        read_ram(ECDSA_SIGN_OUT_R, &mut sign_out_r);
        read_ram(ECDSA_SIGN_OUT_S, &mut sign_out_s);
        info!(
            "sign_out_r: {:#X} sign_out_s: {:#X}",
            sign_out_r, sign_out_s
        );
    } else if result[0] == 0xCBC9 {
        info!("Error in computation: {:#X}", result);
    } else if result[0] == 0xA3B7 {
        info!("sign_r is zero: {:#X}", result);
    } else if result[0] == 0xF946 {
        info!("sign_s is zero: {:#X}", result);
    }

    // Clear the completion flag
    pka.pka_clrfr().write(|w| w.procendfc().set_bit());

    // Clear any previous error flags
    pka.pka_clrfr().write(|w| {
        w.addrerrfc()
            .set_bit()
            .ramerrfc()
            .set_bit()
            .procendfc()
            .set_bit()
    });

    // Verification
    zero_ram();
    write_ram(ECDSA_VERIFY_N_LEN, &[OPERAND_LENGTH]);
    write_ram(ECDSA_VERIFY_P_LEN, &[OPERAND_LENGTH]);
    write_ram(ECDSA_VERIFY_A_SIGN, &[A_SIGN]);
    write_ram(ECDSA_VERIFY_A, &A);
    // write_ram(ECDSA_VERIFY_B, &B);

    write_ram(ECDSA_VERIFY_P, &P);
    write_ram(ECDSA_VERIFY_X, &BASE_POINT_X);
    write_ram(ECDSA_VERIFY_Y, &BASE_POINT_Y);
    write_ram(ECDSA_VERIFY_XQ, &CURVE_PT_X);
    write_ram(ECDSA_VERIFY_YQ, &CURVE_PT_Y);
    write_ram(ECDSA_VERIFY_R, &sign_out_r);
    write_ram(ECDSA_VERIFY_S, &sign_out_s);
    write_ram(ECDSA_VERIFY_Z, &HASH);
    write_ram(ECDSA_VERIFY_N, &PRIME_ORDER);

    // Configure PKA operation mode and start
    info!("Starting Verify operation...");
    gpio.gpioa_bsrr().write(|w| w.bs12().set_bit()); // set high

    pka.pka_cr().modify(
        |_, w| w.mode().bits(VERIFY).start().set_bit(), // Start the operation
    );

    // Wait for processing to complete - PROCENDF is 1 when done
    // info!("Waiting for operation to complete...");
    while pka.pka_sr().read().procendf().bit_is_clear() {
        asm::nop();
    }
    gpio.gpioa_bsrr().write(|w| w.br12().set_bit());
    info!("Operation complete!");

    // Read the result
    let mut result_verify = [0u32; 1];
    let mut sign_out_r_verify = [0u32; MODULUS_SIZE];
    read_ram(ECDSA_VERIFY_OUT, &mut result_verify);
    if result_verify[0] == 0xD60D {
        info!("No errors: {:#X}", result_verify);
        read_ram(ECDSA_VERIFY_SIGN_OUT_R, &mut sign_out_r_verify);
        info!("sign_out_r_verify: {:#X}", sign_out_r_verify);
    } else if result_verify[0] == 0xA3B7 {
        info!("Invalid signature: {:#X}", result_verify);
    }

    loop {}
}
