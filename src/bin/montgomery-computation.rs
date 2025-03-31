#![no_std]
#![no_main]

// Reference Manual: file:///C:/Users/elopezpe/OneDrive/Documentos/PhD/micro/stm32eba55cg/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf
// use stm32wba::stm32wba55;
use stm32wba::stm32wba55::{self};
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;
use cortex_m::{asm};
use defmt::info;
// use stm32_metapac::{metadata, pka};
use core::{
    mem::size_of,
    ptr::{read_volatile, write_volatile},
};

const BASE: usize = 0x520C_2000;
const PKA_RAM_OFFSET: usize = 0x400; 
const RAM_BASE: usize = BASE + PKA_RAM_OFFSET;
const RAM_NUM_DW: usize = 667;

const MODULUS_LENGTH_OFFSET: usize = BASE + 0x408;
const MODULUS_OFFSET: usize = BASE + 0x1088;
const RESULT_OFFSET: usize = BASE + 0x620;
const MODULUS_OFFSET_PREVIOUS_LAST: usize = BASE + 0x1084;

// const N_LENGTH: u32 = 4;
// const N: [u32; 1] = [0xD];
// const ARRAY_NUM: usize = 1;

// P-256 curve parameters. Big endian. The first values are the most significant
const N: [u32; 8] = [
    0xffffffff, 0x00000001, 0x00000000, 0x00000000, 
    0x00000000, 0xffffffff, 0xffffffff, 0xffffffff,
];
const R2MODN: [u32; 8] = [
    0xFFFFFFFC, 0xFFFFFFFC, 0xFFFFFFFB, 0xFFFFFFF9, 
    0xFFFFFFFE, 0x3, 0x5, 0x2
];
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

    // Enable HSI as a stable clock source
    clock.rcc_cr().modify(|_, w| w
    .hseon().set_bit()
    );
    while clock.rcc_cr().read().hserdy().bit_is_clear() {
        asm::nop();
    }

    // Enable RNG clock. Select the source clock. Select the AHB clock
    clock.rcc_ccipr2().write(|w| w.rngsel().b_0x2());
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
        .nistc().clear_bit()   // Hardware default values for NIST compliant RNG
        .ced().clear_bit()     // Clock error detection enabled
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
    // info!("RNG enabled successfully");

    // Enable PKA peripheral clock via RCC_AHB2ENR register
    // PKA peripheral is located on AHB2
    clock.rcc_ahb2enr().modify(|_, w| w.pkaen().set_bit());

    // Reset PKA before enabling (sometimes helps with initialization)
    pka.pka_cr().modify(|_, w| w.en().clear_bit());
    for _ in 0..10 {
        asm::nop();
    }

    // Enable PKA peripheral
    pka.pka_cr().write(|w| w
        .en().set_bit()
        .mode().bits(0x01)
    );
 
    // Wait for PKA to initialize
    while pka.pka_sr().read().initok().bit_is_clear() {
        asm::nop();
    }
    info!("PKA initialized successfully!");

    // Clear any previous error flags
    pka.pka_clrfr().write(|w| w
        .addrerrfc().set_bit()
        .ramerrfc().set_bit()
        .procendfc().set_bit()
    );

    // Write the values - using 32-bit words
    zero_ram();
    write_ram(MODULUS_LENGTH_OFFSET, &[OPERAND_LENGTH]);
    write_ram(MODULUS_OFFSET, &N);
    
    // Check the values 
    let mut buf = [0u32; WORD_LENGTH];
    read_ram(MODULUS_OFFSET, &mut buf);
    info!("modulus: {:#X}", buf);

    // Configure PKA operation mode and start
    info!("Starting PKA operation...");
    pka.pka_cr().modify(|_, w| w
        .mode().bits(0x01)
        .start().set_bit()  // Start the operation
    );

    // Wait for processing to complete - PROCENDF is 1 when done
    info!("Waiting for operation to complete...");
    while pka.pka_sr().read().procendf().bit_is_clear() {
        asm::nop();
    }
    info!("Operation complete!");

    // Read the result
    let mut result = [0u32; WORD_LENGTH];
    read_ram(RESULT_OFFSET, &mut result);
    info!("Montomery parameter for N: {:#X} is {:#X}", N, result);
    
    // Clear the completion flag
    pka.pka_clrfr().write(|w| w.procendfc().set_bit());

    loop {}
}