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
const MODE: u8 = 0x0d;
const RAM_NUM_DW: usize = 667;

// PKA RAM locations for multiplication
const OPERAND_LENGTH_OFFSET: usize = BASE + 0x400 ;
const MODULUS_LENGTH_OFFSET: usize = BASE + 0x408 ;
const OPERAND_A_OFFSET: usize = BASE + 0xA50;
const MODULUS_OFFSET: usize = BASE + 0xC68;
const RESULT_OFFSET: usize = BASE + 0xE78;

// Big endian. LS comes last
// const N: [u32; 8] = [
//     0xffffffff, 0x00000001, 0x00000000, 0x00000000, 
//     0x00000000, 0xffffffff, 0xffffffff, 0xffffffff,
// ];

// const A: [u32; 8] = [
//     0xffffffff, 0x00000001, 0x00000000, 0x00000000, 
//     0x00000000, 0xffffffff, 0xffffffff, 0xfffffffe,
// ];

// const B: [u32; 10] = [
//     0x00000001, 0x00000000, 0x00000000, 0x00000000, 
//     0x00000000, 0x00000000, 0x00000000, 0x00000000,
//     0x00000000, 0x00000000
// ];

// const OPERAND_LENGTH: u32 = 8 * 32;
// const WORD_LENGTH: usize = (OPERAND_LENGTH as usize)/32;    

const A: [u32; 2] = [0xf000000f, 0x00000002];           
const N: [u32; 2] = [0xf0000000, 0x00000001];
const OPERAND_LENGTH: u32 = 2 * 32;
const MODULUS_LENGTH: u32 = 2 * 32;
const WORD_LENGTH: usize = 2; //(OPERAND_LENGTH as usize)/32;    

unsafe fn write_ram(offset: usize, buf: &[u32]) {
    debug_assert_eq!(offset % 4, 0);
    debug_assert!(offset + buf.len() * size_of::<u32>() < 0x520C_33FF);
    
    buf.iter().rev().enumerate().for_each(|(idx, &dw)| {
        let addr = offset + idx * size_of::<u32>();
        info!("Writing: Address {:#X}, Value {:#X}", addr, dw);
        write_volatile(addr as *mut u32, dw);
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
    // .hsikeron().set_bit()
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
    // info!("PKA initialized successfully!");

    // Clear any previous error flags
    pka.pka_clrfr().write(|w| w
        .addrerrfc().set_bit()
        .ramerrfc().set_bit()
        .procendfc().set_bit()
    );

    // First compute AR = A x r2modn mod n
    zero_ram();
    write_ram(OPERAND_LENGTH_OFFSET, &[OPERAND_LENGTH]);
    write_ram(MODULUS_LENGTH_OFFSET, &[MODULUS_LENGTH]);
    write_ram(OPERAND_A_OFFSET, &A);
    write_ram(MODULUS_OFFSET, &N);

    // Check the values 
    let mut buf = [0u32; WORD_LENGTH + 3];
    read_ram(OPERAND_A_OFFSET, &mut buf);

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

    // Add error checking after PKA operations
    if pka.pka_sr().read().addrerrf().bit_is_set() {
        info!("Address Error detected");
    } else if pka.pka_sr().read().ramerrf().bit_is_set() {
        info!("RAM Error detected");
    } else {
        info!("No errors");
    }
    
    // Read the result
    let mut AR = [0u32; WORD_LENGTH];
    read_ram(RESULT_OFFSET, &mut AR);
    info!("A({:#X}) reduced = A({:#X}) (mod {:#X})", A, AR, N,);
    
    // Clear the completion flag
    pka.pka_clrfr().write(|w| w.procendfc().set_bit());

    loop {}
}