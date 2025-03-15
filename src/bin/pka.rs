#![no_std]
#![no_main]

// Reference Manual: file:///C:/Users/elopezpe/OneDrive/Documentos/PhD/micro/stm32eba55cg/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf
// use stm32wba::stm32wba55;
use stm32wba::stm32wba55::{self, RAMCFG};
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;
use cortex_m::asm;
use defmt::info;

// PKA RAM locations - these are already offsets from PKA base address
const PKA_RAM_OFFSET: u32 = 0x400; 
const OPERAND_LENGTH_OFFSET: u32 = 0x408 - 0x400; // Relative to PKA_RAM_OFFSET
const OPERAND_A_OFFSET: u32 = 0xA50 - 0x400;
const OPERAND_B_OFFSET: u32 = 0xC68 - 0x400;
const MODULUS_OFFSET: u32 = 0x1088 - 0x400;
const RESULT_OFFSET: u32 = 0xE78 - 0x400;

const OPERAND_LENGTH: u32 = 64;
const A: u32 = 3;      // First operand
const B: u32 = 11;     // Second operand
const N: u32 = 13;     // Modulus for addition

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
        asm::nop(); // Small delay
    }

    // Enable PKA peripheral
    pka.pka_cr().write(|w| w
        .en().set_bit()
        .mode().bits(0x0E)  // Modular addition mode
    );

    // Read back and print the value of EN bit
    // info!("PKA enabled: {}", pka.pka_cr().read().en().bit_is_set());
 
    // Wait for PKA to initialize
    // info!("INITOK bit set: {:?}", pka.pka_sr().read().initok().bit_is_set());
    while pka.pka_sr().read().initok().bit_is_clear() {
        asm::nop();
    }
    info!("PKA initialized successfully!");

    // PKA RAM base address
    let pka_base = &p.PKA as *const _ as u32;
    let pka_ram_base = pka_base + PKA_RAM_OFFSET;
    // Access PKA RAM as 32-bit words
    let pka_ram = pka_ram_base as *mut u32;
    
    info!("PKA peripheral base address: {:#08x}", pka_base);
    info!("PKA RAM base address: {:#08x}", pka_ram_base);
    
    // Calculate correct offsets in 32-bit words (divide by 4 instead of 8)
    let length_addr = pka_ram.wrapping_add((OPERAND_LENGTH_OFFSET / 4) as usize);
    let a_addr = pka_ram.wrapping_add((OPERAND_A_OFFSET / 4) as usize);
    let b_addr = pka_ram.wrapping_add((OPERAND_B_OFFSET / 4) as usize);
    let modulus_addr = pka_ram.wrapping_add((MODULUS_OFFSET / 4) as usize);
    let result_addr = pka_ram.wrapping_add((RESULT_OFFSET / 4) as usize);

    // // Debug to verify addresses
    // info!("Operand Length address: {:#08x}", length_addr as u32);
    // info!("Operand A address: {:#08x}", a_addr as u32);
    // info!("Operand B address: {:#08x}", b_addr as u32);
    // info!("Modulus address: {:#08x}", modulus_addr as u32);

    // Clear any previous error flags
    pka.pka_clrfr().write(|w| w
        .addrerrfc().set_bit()
        .ramerrfc().set_bit()
        .procendfc().set_bit()
    );


    // Write the values - using 32-bit words
    info!("Writing operand length...");
    core::ptr::write_volatile(length_addr, OPERAND_LENGTH);
    
    info!("Writing operand A...");
    core::ptr::write_volatile(a_addr, A);
    core::ptr::write_volatile(a_addr.add(1), 0); // Additional zero word
    
    info!("Writing operand B...");
    core::ptr::write_volatile(b_addr, B);
    core::ptr::write_volatile(b_addr.add(1), 0); // Additional zero word
    
    info!("Writing modulus...");
    core::ptr::write_volatile(modulus_addr, N);
    core::ptr::write_volatile(modulus_addr.add(1), 0); // Additional zero word

    info!("Data loaded");
    info!("ADDRERRF is clear: {}", pka.pka_sr().read().addrerrf().bit_is_clear());

    // Configure PKA operation mode and start
    info!("Starting PKA operation...");
    pka.pka_cr().modify(|_, w| w
        .mode().bits(0x0E)  // Modular addition mode
        .start().set_bit()  // Start the operation
    );

    // Wait for processing to complete - PROCENDF is 1 when done
    info!("Waiting for operation to complete...");
    while pka.pka_sr().read().procendf().bit_is_clear() {
        asm::nop();
    }
    info!("Operation complete!");

    // Read the result
    let result = core::ptr::read_volatile(result_addr);
    info!("Modular Addition: {} + {} (mod {}) = {}", A, B, N, result);
    
    // Clear the completion flag
    pka.pka_clrfr().write(|w| w.procendfc().set_bit());

    loop {}
}