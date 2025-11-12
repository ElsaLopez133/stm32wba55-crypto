#![no_std]
#![no_main]

// Reference Manual: file:///C:/Users/elopezpe/OneDrive/Documentos/PhD/micro/stm32eba55cg/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf
// use stm32wba::stm32wba55;
use stm32wba::stm32wba55::{self};
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;
use cortex_m::asm;
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

// PKA RAM locations
const OPERAND_LENGTH_OFFSET: usize = BASE +  0x408 ;
const OPERAND_A_OFFSET: usize = BASE +  0xA50;
const OPERAND_B_OFFSET: usize = BASE +  0xC68;
const MODULUS_OFFSET: usize = BASE +  0x1088;
const RESULT_OFFSET: usize = BASE +  0xE78;
const MODE: u8 = 0xE;

const N: [u32; 8] = [
    0xffffffff, 0x00000001, 0x00000000, 0x00000000, 
    0x00000000, 0xffffffff, 0xffffffff, 0xffffffff,
];

// const A: [u32; 8] = [
//     0xffffffff, 0x00000001, 0x00000000, 0x00000000, 
//     0x00000000, 0xffffffff, 0xffffffff, 0xfffffffe,
// ];

const A: [u32; 8] = [
    0xC2ED62C5, 0xE9FCF0BA, 0xEAF30BB3, 0x22CE215D, 
    0x6694D545, 0xB235C821, 0x3BD529B5, 0x7A1C5A20
];

// const B: [u32; 8] = [
//     0xffffffff, 0x00000001, 0x00000000, 0x00000000, 
//     0x00000000, 0xffffffff, 0xffffffff, 0xfffffffe,
// ];

const B: [u32; 8] =  [
    0xD1F3A4C8, 0xB66E30F7, 0x8A53E5B7, 0x896AB8A2, 
    0xFFEFC0BD, 0xE45A7A7E, 0x13347157, 0x956C8E2A
];

const OPERAND_LENGTH: u32 = 8 * 32;
const WORD_LENGTH: usize = (OPERAND_LENGTH as usize)/32;

// const OPERAND_LENGTH: u32 = 4;
// const A: u32 = 11;      // First operand
// const B: u32 = 11;     // Second operand
// const N: u32 = 13;     // Modulus for addition


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
    // info!("HSE ready: {}", clock.rcc_cr().read().hserdy().bit_is_set());

    // Enable RNG clock. Select the source clock
    clock.rcc_ccipr2().write(|w| w.rngsel().b_0x2());
    // Enable RNG clock. Select the AHB clock
    clock.rcc_ahb2enr().modify(|_, w| w.rngen().set_bit());
    while clock.rcc_ahb2enr().read().rngen().bit_is_clear() {
        asm::nop();
    }
    // info!("RNG clock enabled: {}", clock.rcc_ahb2enr().read().rngen().bit_is_set());

    // Configure RNG
    // To configure, CONDRST bit is set to 1 in the same access and CONFIGLOCK remains at 0
    rng.rng_cr().write(|w| w
        .rngen().clear_bit()
        .condrst().set_bit()
        .configlock().clear_bit()
        // .clkdiv().b_0x0()    
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
    
    // info!("SEIS bit is 0: {:?}   SECS bit is 0: {:?}   CEIS bit is 0: {:?}    CECS bit is 0: {:?}", 
    //     rng.rng_sr().read().seis().bit_is_clear(), 
    //     rng.rng_sr().read().secs().bit_is_clear(), 
    //     rng.rng_sr().read().ceis().bit_is_clear(),
    //     rng.rng_sr().read().cecs().bit_is_clear(),
    // );

    // info!("DRDY is ready: {:?}", rng.rng_sr().read().drdy().bit_is_set());
    while rng.rng_sr().read().drdy().bit_is_clear() {
        asm::nop();
    }
    info!("RNG enabled successfully");

    // Enable PKA peripheral clock via RCC_AHB2ENR register
    // PKA peripheral is located on AHB2
    clock.rcc_ahb2enr().modify(|_, w| w.pkaen().set_bit());
    // info!("PKA clock enabled: {}", clock.rcc_ahb2enr().read().pkaen().bit_is_set());

    // Reset PKA before enabling (sometimes helps with initialization)
    pka.pka_cr().modify(|_, w| w.en().clear_bit());
    for _ in 0..10 {
        asm::nop();
    }

    // Enable PKA peripheral
    pka.pka_cr().write(|w| w
        .en().set_bit()
        // .mode().bits(MODE)
    );

    // Read back and print the value of EN bit
    // info!("PKA enabled: {}", pka.pka_cr().read().en().bit_is_set());
 
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
    write_ram(OPERAND_LENGTH_OFFSET, &[OPERAND_LENGTH]);
    write_ram(OPERAND_A_OFFSET, &A);
    write_ram(OPERAND_A_OFFSET + (WORD_LENGTH + 1)*4, &[0]); 
    write_ram(OPERAND_B_OFFSET, &B);
    write_ram(OPERAND_B_OFFSET + (WORD_LENGTH + 1)*4, &[0]);
    write_ram(MODULUS_OFFSET, &N);
    write_ram(MODULUS_OFFSET + (WORD_LENGTH + 1)*4, &[0]);

    // Check the values 
    // let mut buf = [0u32; 8];
    // read_ram(length_addr, &mut buf);
    // info!("length: {:?}", buf);
    // read_ram(operand_a_addr, &mut buf);
    // info!("operand_a: {:#X}", buf);
    // read_ram(operand_b_addr, &mut buf);
    // info!("operand_b: {:#X}", buf);
    // read_ram(modulus_addr, &mut buf);
    // info!("modulus: {:#X}", buf);
    // read_ram(result_addr, &mut buf);
    // info!("result: {:#X}", buf);

    // Configure PKA operation mode and start
    info!("Starting PKA operation...");
    pka.pka_cr().modify(|_, w| w
        .mode().bits(MODE)
        .start().set_bit()  // Start the operation
    );

    // Wait for processing to complete - PROCENDF is 1 when done
    info!("Waiting for operation to complete...");
    while pka.pka_sr().read().procendf().bit_is_clear() {
        asm::nop();
    }
    info!("Operation complete!");
    // info!("ADDRERRF is clear: {}", pka.pka_sr().read().addrerrf().bit_is_clear());
    // info!("RAMERREF is clear: {}", pka.pka_sr().read().ramerrf().bit_is_clear());

    // Read the result
    let mut result = [0u32; 8];
    read_ram(RESULT_OFFSET, &mut result);
    info!("Operation: {:#X} + {:#X} (mod {:#X}) = {:#X}", A, B, N, result);
    info!("Result: {:#X}", result);
    
    // Clear the completion flag
    pka.pka_clrfr().write(|w| w.procendfc().set_bit());

    loop {}
}