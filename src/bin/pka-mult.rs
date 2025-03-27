#![no_std]
#![no_main]
// Test vectors: https://github.com/scogliani/ecc-test-vectors?tab=readme-ov-file
// Reference Manual: file:///C:/Users/elopezpe/OneDrive/Documentos/PhD/micro/stm32eba55cg/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf
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
const MODE: u8 = 0x10;

// PKA RAM locations for multiplication
const OPERAND_LENGTH_OFFSET: u32 = 0x408 ;
const OPERAND_A_OFFSET: u32 = 0xA50;
const OPERAND_B_OFFSET: u32 = 0xC68;
const MODULUS_OFFSET: u32 = 0x1088;
const RESULT_OFFSET: u32 = 0xE78;
const MONTGOMERY_OFFSET: u32 = 0x620;

const A: [u32; 1] = [0x00000004];               
const B: [u32; 1] = [0x00000002];            
const N: [u32; 1] = [0x0000000D];
const OPERAND_LENGTH: u32 = 1 * 32;
const WORD_LENGTH: usize = 1; //(OPERAND_LENGTH as usize)/32;    
const R2MODN: [u32; 1] = [0x30000000];   

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

    let length_addr = BASE + OPERAND_LENGTH_OFFSET as usize;
    let operand_a_addr = BASE + OPERAND_A_OFFSET as usize;
    let operand_b_addr = BASE + OPERAND_B_OFFSET as usize;
    let modulus_addr = BASE + MODULUS_OFFSET as usize;
    let result_addr = BASE + RESULT_OFFSET as usize;
    let montgomery_addr = BASE + MONTGOMERY_OFFSET as usize;

    // Clear any previous error flags
    pka.pka_clrfr().write(|w| w
        .addrerrfc().set_bit()
        .ramerrfc().set_bit()
        .procendfc().set_bit()
    );


    // Write the values - using 32-bit words
    write_ram(length_addr, &[OPERAND_LENGTH]);
    write_ram(operand_a_addr, &A);
    write_ram(modulus_addr + (WORD_LENGTH + 1)*4, &[0]); 
    write_ram(operand_b_addr, &B);
    write_ram(modulus_addr + (WORD_LENGTH + 1)*4, &[0]);
    write_ram(modulus_addr, &N);
    write_ram(modulus_addr + (WORD_LENGTH + 1)*4, &[0]);
    write_ram(montgomery_addr, &R2MODN);
    write_ram(montgomery_addr + (WORD_LENGTH + 1)*4, &[0]); 

    // Check the values 
    let mut buf = [0u32; WORD_LENGTH];
    read_ram(operand_a_addr, &mut buf);
    info!("operand_a: {:#X}", buf);
    read_ram(operand_b_addr, &mut buf);
    info!("operand_b: {:#X}", buf);
    read_ram(modulus_addr, &mut buf);
    info!("modulus: {:#X}", buf);
    read_ram(montgomery_addr, &mut buf);
    info!("montogmery: {:#X}", buf);

    // Configure PKA operation mode and start
    info!("Starting PKA operation...");
    pka.pka_cr().modify(|_, w| w
        .mode().bits(0x10)
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
    read_ram(result_addr, &mut result);
    info!("Operation: {:#X} * {:#X} (mod {:#X}) = {:#X}", A, B, N, result);
    
    // Clear the completion flag
    pka.pka_clrfr().write(|w| w.procendfc().set_bit());

    loop {}
}