#![no_std]
#![no_main]

// use stm32wba55_pac as stm32wba55;
use stm32wba::stm32wba55::{self, hash::hash_din};
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;

const SHA256_DIGEST_LEN: usize = 32;

const MESSAGE_LEN: usize = 128;
// static MESSAGE: [u8; MESSAGE_LEN] = [0xfa; MESSAGE_LEN];
static MESSAGE: [u8; 3] = *b"abc";

#[entry]
unsafe fn main() -> ! {
    // Access peripherals via PAC (Peripheral Access Crate)
    let p = stm32wba55::Peripherals::take().unwrap();
    let hash = p.HASH;
    let pka = p.PKA;
    
    // Set the input source for DMA
    hash.hash_din().write(|w| unsafe { w.bits(MESSAGE.as_ptr() as u32) });
    hash.hash_din().write(|w| unsafe { w.bits(MESSAGE.len() as u32) });

    hash.hash_cr().write(|w| w
        .algo().sha256()  // Select SHA-256 algorithm
        .datatype().bytes32()  // 32-bit data type
        .init().set_bit()  // Initialize hash processor
    );
    // hash.hash_cr().write(|w| w.mode().sha256());

    // Set DCAL bit to start digest calculation
    hash.hash_str().write(|w| w.dcal().set_bit());

    // Wait for BUSY flag to clear
    while hash.hash_sr().read().busy().bit_is_set() {}

    // HASH_HR0 to HASH_HR7 registers return the SHA2-256 digest result
    // file:///C:/Users/elopezpe/Downloads/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf pg 847
    // Read hash result from HASH_HR0-HASH_HR7
    let result = [
        hash.hash_hr0().read().bits(),
        hash.hash_hr1().read().bits(),
        hash.hash_hr2().read().bits(),
        hash.hash_hr3().read().bits(),
        hash.hash_hr4().read().bits(),
        hash.hash_hr5().read().bits(),
        hash.hash_hr6().read().bits(),
        hash.hash_hr7().read().bits(),
    ];

    loop{}
    
}
