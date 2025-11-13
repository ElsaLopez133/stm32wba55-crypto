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
// static MESSAGE: [u32; 1] = [0x00616263];

#[entry]
unsafe fn main() -> ! {
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
    
    // clock.rcc_ahb2enr().modify(|_, w| w.hashen().set_bit());
    // clock.rcc_ahb2enr().modify(|_, w| w.gpioaen().set_bit());

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

    info!("Starting SHA-256 hash calculation");

    // Reset HASH peripheral
    hash.hash_cr().write(|w| w.init().set_bit());
    while hash.hash_cr().read().init().bit_is_set() {
        asm::nop();
    }

    // // Read and log initial HASH_CR value
    // let cr_value = hash.hash_cr().read().bits();
    // let algo = hash.hash_cr().read().algo().bits();
    // let datatype = hash.hash_cr().read().datatype().bits();
    // info!("Initial HASH_CR: 0x{:b}, ALGO: {:b}, DATATYPE: {:b}", cr_value, algo, datatype);
    

    // Configure for SHA-256 mode with byte-swapping
    hash.hash_cr().write(|w| w
        .algo().bits(0b11)      // SHA-256 algorithm
        .mode().bit(false)      // Hash mode (not HMAC)
        .datatype().bits(0b10)  // 8-bit data with byte swapping
        .dmae().clear_bit()     // No DMA
        .init().set_bit()       // Complete the initialization by setting to 1 the INIT bit in HASH_CR (pg835)
    );

    // // Read and log updated HASH_CR value
    // let cr_value = hash.hash_cr().read().bits();
    // let algo = hash.hash_cr().read().algo().bits();
    // let datatype = hash.hash_cr().read().datatype().bits();
    // info!("Configured HASH_CR: 0x{:b}, ALGO: {:b}, DATATYPE: {:b}", cr_value, algo, datatype);

    // Check that the peripheral is ready (not busy)
    if hash.hash_sr().read().busy().bit_is_set() {
        info!("WARNING: HASH peripheral is busy before data input!");
    } else {
        info!("HASH peripheral is ready for data input");
    }

    // First word: byte-swapped "abc" 
    // 0xUU636261 (where 'U' is don't care)
    let word = 0x636261;
    hash.hash_din().write(|w| w.bits(word));

    // Set NBLW to 24 (original message length)
    hash.hash_str().write(|w| w.nblw().bits(24));

    // Begin hash computation
    gpio.gpioa_bsrr().write(|w| w.bs12().set_bit());

    // Start padding and digest computation
    hash.hash_str().write(|w| w.dcal().set_bit());

    // Wait for digest calculation to complete
    while hash.hash_sr().read().busy().bit_is_set() {
        asm::nop();
    }
    // Also check that DCAL bit has been cleared by hardware
    while hash.hash_sr().read().dcis().bit_is_clear() {
        asm::nop();
    }
    // end of hash
    gpio.gpioa_bsrr().write(|w| w.br12().set_bit());

    info!("Hash calculation complete");


    // let hr0 = hash.hash_hr0().read().bits();
    // info!("HR0 = {:08x}", hr0);

    // let hr1 = hash.hash_hr1().read().bits();
    // info!("HR1 = {:08x}", hr1);

    // let hr2 = hash.hash_hr2().read().bits();
    // info!("HR2 = {:08x}", hr2);

    // let hr3 = hash.hash_hr3().read().bits();
    // info!("HR3 = {:08x}", hr3);

    // let hr4 = hash.hash_hr4().read().bits();
    // info!("HR4 = {:08x}", hr4);

    // let hr5 = hash.hash_hr5().read().bits();
    // info!("HR5 = {:08x}", hr5);

    // let hr6 = hash.hash_hr6().read().bits();
    // info!("HR6 = {:08x}", hr6);

    // let hr7 = hash.hash_hr7().read().bits();
    // info!("HR7 = {:08x}", hr7);


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
    // info!("SHA-256 hash:");
    info!("{:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x} {:08x}", 
        hash_result[0], hash_result[1], hash_result[2], hash_result[3],
        hash_result[4], hash_result[5], hash_result[6], hash_result[7]);

    info!("Expected hash for 'abc': ba7816bf 8f01cfea 414140de 5dae2223 b00361a3 96177a9c b410ff61 f20015ad");

    loop {}
}