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
// static MESSAGE: [u8; 3] = *b"abc";
// static MESSAGE: [u32; 1] = [0x00616263];
const MESSAGE: [u32; 8] = [ 
    0xA41A41A1, 0x2A799548, 0x211C410C, 0x65D8133A,
    0xFDE34D28, 0xBDD542E4, 0xB680CF28, 0x99C8A8C4
];

// Static variable to store the hash result
static mut HASH_RESULT: [u32; 8] = [0; 8];
static mut HASH_RESULT_SWAPPED: [u32; 8] = [0; 8];
#[entry]
unsafe fn main() -> ! {
    // Access peripherals via PAC
    let p = stm32wba55::Peripherals::take().unwrap();
    let hash = &p.HASH;
    let clock = &p.RCC;
    let gpio = &p.GPIOA;

    // Enable HASH peripheral clock via RCC_AHB2ENR register
    // HASH peripheral is located on AHB2
    clock.rcc_ahb2enr().modify(|_, w| {
        w.hashen().set_bit();
        w.gpioaen().set_bit()
    });
    
    // set pin to putput mode
    gpio.gpioa_moder().modify(|_, w| unsafe { w.mode12().bits(0b01) }); // PA15 as output
    // set output type to push-pull
    gpio.gpioa_otyper().modify(|_, w| w.ot12().clear_bit());
    // set speed to low
    gpio.gpioa_ospeedr().modify(|_, w| unsafe { w.ospeed12().bits(0b00) });
    // no pull-up/pull-down
    gpio.gpioa_pupdr().modify(|_, w| unsafe { w.pupd12().bits(0b00) });
    // set initial state to low
    gpio.gpioa_bsrr().write(|w| w.br12().set_bit());
    
    // Reset HASH peripheral
    hash.hash_cr().write(|w| w.init().set_bit());
    while hash.hash_cr().read().init().bit_is_set() {
        asm::nop();
    }
    info!("HASH peripheral initialized");


    // Configure for SHA-256 mode. HASH_CR pg844 Reference Manual
    hash.hash_cr().write(|w| w
        .algo().bits(0b11)       // Set to SHA2-256 algorithm (11)
        .mode().bit(false)       // 0: Hash mode (not HMAC)
        .datatype().bits(0b00)   // 10: 32-bit data
        .dmae().clear_bit()      // Bit 3: No DMA (0)
        .init().set_bit()        // Complete the initialization by setting to 1 the INIT bit in HASH_CR (pg835)
    );

    // Check that the peripheral is ready (not busy)
    if hash.hash_sr().read().busy().bit_is_set() {
        info!("WARNING: HASH peripheral is busy before data input!");
    } else {
        info!("HASH peripheral is ready for data input");
    }

    // let word = 0x61626300;
    for &word in MESSAGE.iter() {
        hash.hash_din().write(|w| w.bits(word));
    }

    // // Pack bytes into a word (big-endian for SHA-256)
    // let mut word = 0u32;
    // for (i, &byte) in MESSAGE.iter().enumerate() {
    //     // Shift existing bits and add new byte
    //     word |= u32::from(byte) << (8 * (3 - (i % 4)));
        
    //     // Write word when we have 4 bytes or at the end of the message
    //     if ((i + 1) % 4 == 0) || (i == MESSAGE.len() - 1) {
    //         // If it's the last word and not a full 4-byte word, add padding
    //         if i == MESSAGE.len() - 1 && MESSAGE.len() % 4 != 0 {
    //             word |= 0x80 >> (8 * (i % 4 + 1));
    //         }
            
    //         info!("Writing word: 0x{:08x}", word);
    //         hash.hash_din().write(|w| w.bits(word));
    //         word = 0;
    //     }
    // }


    // // If message length is not a multiple of 4, ensure proper padding
    // if MESSAGE.len() % 4 != 0 {
    //     // Set NBLW to the number of valid bits in the last word
    //     hash.hash_str().write(|w| w.nblw().bits((MESSAGE.len() as u8 % 4) * 8));
    // }
    // // Start padding and digest computation
    // hash.hash_str().write(|w| w.dcal().set_bit());

    
    // All 256 bits are valid (no partial word)
    hash.hash_str().write(|w| w.nblw().bits(0));

    info!("Starting HASH computation");
    gpio.gpioa_bsrr().write(|w| w.bs12().set_bit());

    hash.hash_str().write(|w| w.dcal().set_bit());
    
    // Wait for busy bit to clear
    while hash.hash_sr().read().busy().bit_is_set() {
        asm::nop();
    }

    // Also check that DCAL bit has been cleared by hardware
    while hash.hash_sr().read().dcis().bit_is_clear() {
        asm::nop();
    }
    gpio.gpioa_bsrr().write(|w| w.br12().set_bit());
    info!("Hash calculation complete");
    
    // Read hash result from HASH_HR0-HASH_HR7
    HASH_RESULT[0] = hash.hash_hr0().read().bits();
    HASH_RESULT[1] = hash.hash_hr1().read().bits();
    HASH_RESULT[2] = hash.hash_hr2().read().bits();
    HASH_RESULT[3] = hash.hash_hr3().read().bits();
    HASH_RESULT[4] = hash.hash_hr4().read().bits();
    HASH_RESULT[5] = hash.hash_hr5().read().bits();
    HASH_RESULT[6] = hash.hash_hr6().read().bits();
    HASH_RESULT[7] = hash.hash_hr7().read().bits();
    

    // info!("Expected hash for 'abc': ba7816bf 8f01cfea 414140de 5dae2223 b00361a3 96177a9c b410ff61 f20015ad");
    // Output the original hash result
    info!("SHA-256 hash (as-is from registers):");
    info!("{:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x}", 
        HASH_RESULT[0], HASH_RESULT[1], HASH_RESULT[2], HASH_RESULT[3],
        HASH_RESULT[4], HASH_RESULT[5], HASH_RESULT[6], HASH_RESULT[7]);

    loop {}
}