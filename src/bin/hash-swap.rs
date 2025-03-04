#![no_std]
#![no_main]

// Reference Manual: file:///C:/Users/elopezpe/OneDrive/Documentos/PhD/micro/stm32eba55cg/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf

// use stm32wba::stm32wba55;
use stm32wba::stm32wba55;
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;
use cortex_m::asm;
use defmt::info;

// For "abc" input, we need to declare it properly
static MESSAGE: [u8; 3] = *b"abc";

#[entry]
unsafe fn main() -> ! {
    let p = stm32wba55::Peripherals::take().unwrap();
    let hash = &p.HASH;
    let clock = &p.RCC;

    // Enable HASH peripheral clock via RCC_AHB2ENR register
    // HASH peripheral is located on AHB2
    clock.rcc_ahb2enr().modify(|_, w| w.hashen().set_bit());

    // Reset HASH peripheral
    hash.hash_cr().write(|w| w.init().set_bit());
    while hash.hash_cr().read().init().bit_is_set() {
        asm::nop();
    }

    // Configure for SHA-256 mode with byte-swapping
    hash.hash_cr().write(|w| w
        .algo().bits(0b11)      // SHA-256 algorithm
        .mode().bit(false)      // Hash mode (not HMAC)
        .datatype().bits(0b10)  // 8-bit data with byte swapping
        .dmae().clear_bit()     // No DMA
        .init().set_bit()     
    );

    // First word: byte-swapped "abc" 
    // 0xUU636261 (where 'U' is don't care)
    let word = 0x636261;
    hash.hash_din().write(|w| w.bits(word));

    // Set NBLW to 24 (original message length)
    hash.hash_str().write(|w| w.nblw().bits(24));

    // Start padding and digest computation
    hash.hash_str().write(|w| w.dcal().set_bit());

    // Wait for digest calculation to complete
    while hash.hash_sr().read().busy().bit_is_set() {
        asm::nop();
    }

    // Read final hash
    let hash_result = [
        hash.hash_hr0().read().bits(),
        hash.hash_hr1().read().bits(),
        hash.hash_hr2().read().bits(),
        hash.hash_hr3().read().bits(),
        hash.hash_hr4().read().bits(),
        hash.hash_hr5().read().bits(),
        hash.hash_hr6().read().bits(),
        hash.hash_hr7().read().bits(),
    ];

    // Expected hash for "abc"
    info!("SHA-256 hash:");
    info!("{:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x}", 
        hash_result[0], hash_result[1], hash_result[2], hash_result[3],
        hash_result[4], hash_result[5], hash_result[6], hash_result[7]);

    info!("Expected hash for 'abc': ba7816bf 8f01cfea 414140de 5dae2223 b00361a3 96177a9c b410ff61 f20015ad");

    loop {}
}