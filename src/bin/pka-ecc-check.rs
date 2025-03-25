#![no_std]
#![no_main]

// Reference Manual: file:///C:/Users/elopezpe/OneDrive/Documentos/PhD/micro/stm32eba55cg/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf
// use stm32wba::stm32wba55;
use stm32wba::stm32wba55::{self};
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;
use cortex_m::asm;
use defmt::info;
use core::{
    mem::size_of,
    ptr::{read_volatile, write_volatile},
};

const BASE: usize = 0x520C_2000;
const PKA_RAM_OFFSET: usize = 0x400; 
const RAM_BASE: usize = BASE + PKA_RAM_OFFSET;
const MODE: u8 = 0x28;

// PKA RAM locations for exponentiation
const MODULUS_LENGTH_OFFSET: u32 = 0x408;
const COEF_A_SIGN_OFFSET: u32 = 0x410;
const COEF_A_OFFSET: u32 = 0x418;
const COEF_B_OFFSET: u32 = 0x520;
const MODULUS_OFFSET: u32 = 0x470;
const POINT_X_OFFSET: u32 = 0x578;
const POINT_Y_OFFSET: u32 = 0x5D0;
const MONTGOMERY_OFFSET: u32 = 0x4C8;
const RESULT_OFFSET: u32 = 0x680;

const A_SIGN: u32 = 0x1;
const A: [u32; 8] = [
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 
    0x00000000, 0x00000000, 0x00000000, 0x00000003,
];
const N: [u32; 8] = [
    0xffffffff, 0x00000001, 0x00000000, 0x00000000, 
    0x00000000, 0xffffffff, 0xffffffff, 0xffffffff,
];

const B: [u32; 8] = [
    0x5ac635d8, 0xaa3a93e7, 0xb3ebbd55, 0x769886bc,
    0x651d06b0, 0xcc53b0f6, 0x3bce3c3e, 0x27d2604b
];

const BASE_POINT_X: [u32; 8] = [
    0x6b17d1f2, 0xe12c4247, 0xf8bce6e5, 0x63a440f2, 
    0x77037d81, 0x2deb33a0, 0xf4a13945, 0xd898c296,
];

const BASE_POINT_Y: [u32; 8] = [
    0x4fe342e2, 0xfe1a7f9b, 0x8ee7eb4a, 0x7c0f9e16, 
    0x2bce3357, 0x6b315ece, 0xcbb64068, 0x37bf51f5,
];

const PRIME_ORDER: [u32; 8] = [
    0xffffffff, 0x00000000, 0xffffffff, 0xffffffff, 
    0xbce6faad, 0xa7179e84, 0xf3b9cac2, 0xfc632551,
];

const SCALAR: u32 = 0x1;

const R2MODN: [u32; 8] = [
    0xFFFFFFFC, 0xFFFFFFFC, 0xFFFFFFFB, 0xFFFFFFF9, 
    0xFFFFFFFE, 0x3, 0x5, 0x2
];

// const N: [u32; 8] = [
//     0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0x00000000,
//     0x00000000, 0x00000000, 0x00000001, 0xFFFFFFFF,
// ];

// const B: [u32; 8] = [
//     0x27D2604B, 0x3BCE3C3E, 0xCC53B0F6, 0x651D06B0,
//     0x769886BC, 0xB3EBBD55, 0xAA3A93E7, 0x5AC635D8
// ];

// const A: [u32; 8] = [
//     0xFFFFFFFC, 0x00000001, 0x00000000, 0x00000000,
//     0x00000000, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF
// ];

// static POINT_X: [u32; 8] = [
//     0x6B17D1F2, 0xE12C4247, 0xF8BCE6E5, 0x63A440F2,
//     0x77037D81, 0x2DEB33A0, 0xF4A13945, 0xD898C296,
// ];

// static POINT_Y: [u32; 8] = [
//     0x4FE342E2, 0xFE1A7F9B, 0x8EE7EB4A, 0x7C0F9E16,
//     0x2BCE3357, 0x6B315ECE, 0xCBB64068, 0x37BF51F5
// ];

// const R2MODN: [u32; 8] = [
//     0x00000002, 0x00000000, 0xFFFFFFFA, 0x00000004, 
//     0xFFFFFFFB, 0xFFFFFFFF, 0x00000008, 0xFFFFFFFC
// ];

const OPERAND_LENGTH: u32 = 8 * 32;
const WORD_LENGTH: usize = (OPERAND_LENGTH as usize)/32;   


unsafe fn write_ram(offset: usize, buf: &[u32]) {
    debug_assert_eq!(offset % 4, 0);
    debug_assert!(offset + buf.len() * size_of::<u32>() < 0x520C_33FF);
    buf.iter().rev().enumerate().for_each(|(idx, &dw)| {
        write_volatile((offset + idx * size_of::<u32>()) as *mut u32, dw)
    });
}

unsafe fn read_ram(offset: usize, buf: &mut [u32]) {
    debug_assert_eq!(offset % 4, 0);
    debug_assert!(offset + buf.len() * size_of::<u32>() < 0x520C_33FF);
    buf.iter_mut().rev().enumerate().for_each(|(idx, dw)| {
        *dw = read_volatile((offset + idx * size_of::<u32>()) as *const u32);
    });
}


#[entry]
unsafe fn main() -> ! {
    let p = stm32wba55::Peripherals::take().unwrap();
    let pka = &p.PKA;
    let clock = &p.RCC;
    let rng = &p.RNG;

    // Enable HSI as a stable clock source
    clock.rcc_cr().modify(|_, w| w
    .hseon().set_bit()
    // .hsikeron().set_bit()
    );
    while clock.rcc_cr().read().hserdy().bit_is_clear() {
        asm::nop();
    }

    // Enable RNG clock. Select the source clock
    clock.rcc_ccipr2().write(|w| w.rngsel().b_0x2());
    // Enable RNG clock. Select the AHB clock
    clock.rcc_ahb2enr().modify(|_, w| w.rngen().set_bit());
    while clock.rcc_ahb2enr().read().rngen().bit_is_clear() {
        asm::nop();
    }

    // Configure RNG
    // To configure, CONDRST bit is set to 1 in the same access and CONFIGLOCK remains at 0
    rng.rng_cr().write(|w| w
        .rngen().clear_bit()
        .condrst().set_bit()
        .configlock().clear_bit() 
        .nistc().clear_bit()   
        .ced().clear_bit() 
    );

    // First clear CONDRST while keeping RNGEN disabled
    rng.rng_cr().modify(|_, w| w
        .condrst().clear_bit()
    );

    // Then enable RNG in a separate step
    rng.rng_cr().modify(|_, w| w
        .rngen().set_bit()
        .ie().set_bit()
    );
    
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
    pka.pka_cr().write(|w| w
        .en().set_bit()
        .mode().bits(MODE)
    );
 
    // Wait for PKA to initialize
    while pka.pka_sr().read().initok().bit_is_clear() {
        asm::nop();
    }
    info!("PKA initialized successfully!");

    let modulus_length_addr = BASE + MODULUS_LENGTH_OFFSET as usize;
    let coef_a_sign_addr = BASE + COEF_A_SIGN_OFFSET as usize;
    let coef_a_addr = BASE + COEF_A_OFFSET as usize;
    let coef_b_addr = BASE + COEF_B_OFFSET as usize;
    let modulus_addr = BASE + MODULUS_OFFSET as usize;
    let result_addr = BASE + RESULT_OFFSET as usize;
    let point_x_addr = BASE + POINT_X_OFFSET as usize;
    let point_y_addr = BASE + POINT_Y_OFFSET as usize;
    let montgomery_addr = BASE + MONTGOMERY_OFFSET as usize;

    // Clear any previous error flags
    pka.pka_clrfr().write(|w| w
        .addrerrfc().set_bit()
        .ramerrfc().set_bit()
        .procendfc().set_bit()
    );


    // Write the values - using 32-bit words
    write_ram(modulus_length_addr, &[OPERAND_LENGTH]);
    write_ram(coef_a_sign_addr, &[A_SIGN]);
    
    write_ram(coef_a_addr, &A);
    write_ram(coef_a_addr + 4, &[0]);
    write_ram(coef_b_addr, &B);
    write_ram(coef_b_addr + 4, &[0]); 
    write_ram(modulus_addr, &N);
    write_ram(modulus_addr + 4, &[0]); 
    write_ram(point_x_addr, &BASE_POINT_X);
    write_ram(point_x_addr + 4, &[0]); 
    write_ram(point_y_addr, &BASE_POINT_Y);
    write_ram(point_y_addr + 4, &[0]); 
    // write_ram(montgomery_addr, &R2MODN);
    // write_ram(montgomery_addr + 4, &[0]); 

    // Check the values 
    let mut buf = [032; WORD_LENGTH];
    read_ram(coef_a_addr, &mut buf);
    info!("A: {:#X}", buf);
    read_ram(coef_b_addr, &mut buf);
    info!("B: {:#X}", buf);
    read_ram(modulus_addr, &mut buf);
    info!("modulus: {:#X}", buf);
    read_ram(point_x_addr, &mut buf);
    info!("POINT_X: {:#X}", buf);
    read_ram(point_y_addr, &mut buf);
    info!("POINT_Y: {:#X}", buf);

    // Configure PKA operation mode and start
    info!("Starting PKA operation...");
    pka.pka_cr().modify(|_, w| w
        .mode().bits(MODE)
        .start().set_bit()
    );

    // Wait for processing to complete - PROCENDF is 1 when done
    info!("Waiting for operation to complete...");
    while pka.pka_sr().read().procendf().bit_is_clear() {
        asm::nop();
    }
    info!("Operation complete!");

    // Read the result
    let mut result = [0u32; 1];
    read_ram(result_addr, &mut result);
    if result[0] == 0xD60D {
        info!("Point on curve ({:#X})", result);
    } 
    if result[0] == 0xA3B7 {
        info!("Point not on curve ({:#X})", result);
    } 
    if result[0] == 0xF946 {
        info!("X or Y coordinate is not smaller than N ({:#X})", result);
    } 
    
    // Clear the completion flag
    pka.pka_clrfr().write(|w| w.procendfc().set_bit());

    loop {}
}