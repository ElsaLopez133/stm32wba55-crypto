#![no_std]
#![no_main]

// Reference Manual: file:///C:/Users/elopezpe/OneDrive/Documentos/PhD/micro/stm32eba55cg/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf

use stm32wba::stm32wba55;
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;
use cortex_m::asm;
use defmt::info;

const SHA256_DIGEST_LEN: usize = 32;

// For "abc" input, we need to declare it properly
static MESSAGE: [u8; 3] = *b"abc";

// Static variable to store the hash result
static mut HASH_RESULT: [u32; 8] = [0; 8];
// Alternatively for byte access:
// static mut HASH_RESULT_BYTES: [u8; SHA256_DIGEST_LEN] = [0; SHA256_DIGEST_LEN];

#[entry]
unsafe fn main() -> ! {
    // Access peripherals via PAC
    let p = stm32wba55::Peripherals::take().unwrap();
    let hash = &p.HASH;

    info!("Starting SHA-256 hash calculation");
    
    // Reset HASH peripheral
    hash.hash_cr().modify(|_, w| w.init().set_bit());
    info!("HASH peripheral initialized");

    // Read and log initial HASH_CR value
    let cr_value = hash.hash_cr().read().bits();
    info!("Initial HASH_CR value: 0x{:08x}", cr_value);
    
    // Configure for SHA-256 mode. HASH_CR pg844 Reference Manual
    hash.hash_cr().modify(|_, w| w
        .algo().bits(0b11)        // Set to SHA2-256 algorithm (11)
        .mode().bit(false)        // 0: Hash mode (not HMAC)
        .datatype().bits(0b10)    // 10: 8-bit data (bytes)
    );

    // Read and log updated HASH_CR value
    let cr_value_after = hash.hash_cr().read().bits();
    info!("Configured HASH_CR value: 0x{:08x}", cr_value_after);

    // Check that the peripheral is ready (not busy)
    if hash.hash_sr().read().busy().bit_is_set() {
        info!("WARNING: HASH peripheral is busy before data input!");
    } else {
        info!("HASH peripheral is ready for data input");
    }

    // // Send message data byte by byte
    // for &byte in MESSAGE.iter() {
    //     hash.hash_din().write(|w| w.bits(u32::from(byte)));
    // }
    // Send message data byte by byte
    info!("Input message: {:?}", MESSAGE);
    for (i, &byte) in MESSAGE.iter().enumerate() {
        info!("Writing byte[{}]: 0x{:02x} ({})", i, byte, byte as char);
        hash.hash_din().write(|w| w.bits(u32::from(byte)));
        
        // Add a small delay after each write for stability
        for _ in 0..1000 {
            asm::nop();
        }
    }

    info!("All message bytes written to HASH_DIN");
    
    // Check peripheral status before starting calculation
    let sr_value = hash.hash_sr().read().bits();
    info!("HASH_SR before DCAL: 0x{:08x}", sr_value);
    
    // Start digest calculation
    info!("Setting DCAL bit to start digest calculation");
    hash.hash_str().modify(|_, w| w.dcal().set_bit());
    
    // Wait for the hash calculation to complete
    while hash.hash_sr().read().busy().bit_is_set() {
        asm::nop();
    }

    info!("Hash calculation complete");
    
    // Final status register check
    let final_sr = hash.hash_sr().read().bits();
    info!("Final HASH_SR: 0x{:08x}", final_sr);
    
    // Read hash result from HASH_HR0-HASH_HR7 and store it in our static variable
    HASH_RESULT[0] = hash.hash_hr0().read().bits();
    HASH_RESULT[1] = hash.hash_hr1().read().bits();
    HASH_RESULT[2] = hash.hash_hr2().read().bits();
    HASH_RESULT[3] = hash.hash_hr3().read().bits();
    HASH_RESULT[4] = hash.hash_hr4().read().bits();
    HASH_RESULT[5] = hash.hash_hr5().read().bits();
    HASH_RESULT[6] = hash.hash_hr6().read().bits();
    HASH_RESULT[7] = hash.hash_hr7().read().bits();
    
    // This converts the u32 values to bytes in little-endian format
    /*
    let mut byte_index = 0;
    for word in HASH_RESULT.iter() {
        HASH_RESULT_BYTES[byte_index] = (*word & 0xFF) as u8;
        HASH_RESULT_BYTES[byte_index + 1] = ((*word >> 8) & 0xFF) as u8;
        HASH_RESULT_BYTES[byte_index + 2] = ((*word >> 16) & 0xFF) as u8;
        HASH_RESULT_BYTES[byte_index + 3] = ((*word >> 24) & 0xFF) as u8;
        byte_index += 4;
    }
    */
    
    // Optional: Output the result via defmt
    info!("SHA-256 hash: {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x}", 
       HASH_RESULT[0], HASH_RESULT[1], HASH_RESULT[2], HASH_RESULT[3],
       HASH_RESULT[4], HASH_RESULT[5], HASH_RESULT[6], HASH_RESULT[7]);
    
    info!("Expected hash for 'abc': ba7816bf 8f01cfea 414140de 5dae2223 b00361a3 96177a9c b410ff61 f20015ad");


    loop {}
}