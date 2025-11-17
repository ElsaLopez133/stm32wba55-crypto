#![no_std]
#![no_main]

// Reference Manual: file:///C:/Users/elopezpe/OneDrive/Documentos/PhD/micro/stm32eba55cg/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf

// use stm32wba::stm32wba55;
use cortex_m::asm;
use cortex_m_rt::entry;
use defmt::info;
use stm32wba::stm32wba55;
use {defmt_rtt as _, panic_probe as _};

// For "abc" input, we need to declare it properly
// static MESSAGE: [u8; 3] = *b"abc";
static message: [u8; 36] = [
    0xA1, 0x4, 0x41, 0x2B, 0x74, 0x1A, 0x13, 0xD7, 0xBA, 0x4, 0x8F, 0xBB, 0x61, 0x5E, 0x94, 0x38,
    0x6A, 0xA3, 0xB6, 0x1B, 0xEA, 0x5B, 0x3D, 0x8F, 0x65, 0xF3, 0x26, 0x20, 0xB7, 0x49, 0xBE, 0xE8,
    0xD2, 0x78, 0xEF, 0xA9,
];
// static message: [u32; 1] = [0x00616263];
static message_len: usize = 36;
static hash_result: [u8; 32] = [
    0xE8, 0x68, 0x9D, 0x90, 0x91, 0x79, 0x6B, 0xB1, 0xFC, 0x3F, 0x89, 0x4F, 0xFB, 0xC, 0xFE, 0x94,
    0x31, 0xAF, 0xD3, 0x7A, 0x5B, 0x95, 0x1C, 0xA6, 0xE4, 0x4C, 0x84, 0x5C, 0x4F, 0x89, 0xF3, 0xCF,
];

#[entry]
unsafe fn main() -> ! {
    // Access peripherals via PAC
    let p = stm32wba55::Peripherals::take().unwrap();
    let hash = &p.HASH;
    let clock = &p.RCC;

    // Enable HASH peripheral clock via RCC_AHB2ENR register
    // HASH peripheral is located on AHB2
    clock.rcc_ahb2enr().modify(|_, w| w.hashen().set_bit());

    info!("Starting SHA-256 hash calculation");

    // Reset HASH peripheral
    hash.hash_cr().write(|w| w.init().set_bit());
    while hash.hash_cr().read().init().bit_is_set() {
        asm::nop();
    }

    // Configure for SHA-256 mode with byte-swapping
    hash.hash_cr().write(|w| {
        w.algo()
            .bits(0b11) // SHA-256 algorithm
            .mode()
            .bit(false) // Hash mode (not HMAC)
            .datatype()
            .bits(0b00) // 8-bit data with byte swapping
            .dmae()
            .clear_bit() // No DMA
            .init()
            .set_bit()
    });

    // Check that the peripheral is ready (not busy)
    if hash.hash_sr().read().busy().bit_is_set() {
        info!("WARNING: HASH peripheral is busy before data input!");
    } else {
        info!("HASH peripheral is ready for data input");
    }

    // Feed message data to the peripheral
    // Process in 32-bit chunks
    // info!("message: {:#X}", message[..message_len]);
    let full_words = message_len / 4;
    let remainder_bytes = message_len % 4;

    // Write full 32-bit words
    for i in 0..full_words {
        let idx = i * 4;
        let word = (u32::from(message[idx]) << 24)
            | (u32::from(message[idx + 1]) << 16)
            | (u32::from(message[idx + 2]) << 8)
            | u32::from(message[idx + 3]);
        hash.hash_din().write(|w| w.bits(word));
    }

    // Handle remaining bytes in the last partial word, if any
    if remainder_bytes > 0 {
        let mut last_word = 0u32;
        let base_idx = full_words * 4;

        for i in 0..remainder_bytes {
            last_word |= u32::from(message[base_idx + i]) << (24 - (i * 8));
        }
        hash.hash_din().write(|w| w.bits(last_word));
    }

    // Tell the peripheral how many valid bytes are in the last word
    // and start the digest calculation
    hash.hash_str().write(
        |w| {
            w.nblw()
                .bits(remainder_bytes as u8) // Valid bytes in last word
                .dcal()
                .set_bit()
        }, // Start calculation
    );

    // Wait for digest calculation to complete
    while hash.hash_sr().read().busy().bit_is_set() {
        asm::nop();
    }
    // Read hash result and convert to bytes
    let mut result: [u8; 32] = [0u8; 32];

    // Read the 8 hash registers (each 32-bits)
    let hr0 = hash.hash_hr0().read().bits();
    let hr1 = hash.hash_hr1().read().bits();
    let hr2 = hash.hash_hr2().read().bits();
    let hr3 = hash.hash_hr3().read().bits();
    let hr4 = hash.hash_hr4().read().bits();
    let hr5 = hash.hash_hr5().read().bits();
    let hr6 = hash.hash_hr6().read().bits();
    let hr7 = hash.hash_hr7().read().bits();

    // Convert to bytes (be careful about endianness)
    result[0..4].copy_from_slice(&hr0.to_be_bytes());
    result[4..8].copy_from_slice(&hr1.to_be_bytes());
    result[8..12].copy_from_slice(&hr2.to_be_bytes());
    result[12..16].copy_from_slice(&hr3.to_be_bytes());
    result[16..20].copy_from_slice(&hr4.to_be_bytes());
    result[20..24].copy_from_slice(&hr5.to_be_bytes());
    result[24..28].copy_from_slice(&hr6.to_be_bytes());
    result[28..32].copy_from_slice(&hr7.to_be_bytes());

    info!("SHA-256 hash (as-is from registers): {:#X}", result);
    info!("SHA-256 hash (as expected): {:#X}", hash_result);

    assert!(hash_result == result);

    loop {}
}
