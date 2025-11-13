#![no_std]
#![no_main]

// Reference Manual: file:///C:/Users/elopezpe/OneDrive/Documentos/PhD/micro/stm32eba55cg/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf

// use stm32wba::stm32wba55;
use stm32wba::stm32wba55::{self, aes};
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;
use cortex_m::asm;
use defmt::info;

const KEY256: [u32; 8] = [
    0x00112233, 0x44556677, 0x8899aabb, 0xccddeeff,
    0x00010203, 0x04050607, 0x08090a0b, 0x0c0d0e0f,
];

const IV128: [u32; 4] = [
    0x00010203, 0x04050607, 0x08090a0b, 0x0c0d0e0f,
];

const PLAINTEXT_BLOCKS: &[[u32; 4]] = &[
    [0x00112233, 0x44556677, 0x8899aabb, 0xccddeeff],
    // add more 16-byte blocks if needed
];

#[entry]
unsafe fn main() -> ! {
    let p = stm32wba55::Peripherals::take().unwrap();
    let aes = &p.AES;
    let clock = &p.RCC;
    let gpio = &p.GPIOA;

    // Enable AES peripheral clock via RCC_AHB2ENR register
    // AES peripheral is located on AHB2
    clock.rcc_ahb2enr().modify(|_, w| {
        w.aesen().set_bit();
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

    info!("Starting AES calculation");

    // Initialize AES_CR
    // Choose: ECB (CHMOD = 0x0) or CBC (CHMOD = 0x1)
    // MODE = 0x0 (encryption), DATATYPE = 0b00 (32-bit words) or other,
    // KEYSIZE = 0x1 -> 256-bit
    // KMOD = 0x0 (software key load)
    aes.aes_cr().modify(|_, w| {
         w.mode().bits(0b00)       // Encryption mode
         .chmod().bits(0b00)      // ECB mode (no chaining)
         .datatype().bits(0b10)   // 32-bit data no swapping
         .keysize().b_0x1()    // 256-bit key
         .kmod().b_0x0()    
    });

    // Read and log updated HASH_CR value
    let cr_value = aes.aes_cr().read().bits();
    let chmod = aes.aes_cr().read().chmod().bits();
    let datatype = aes.aes_cr().read().datatype().bits();
    let keysize = aes.aes_cr().read().keysize().is_b_0x1();
    info!("Configured AES_CR: 0x{:b}, CHMOD: {:b}, DATATYPE: {:b}, KEYSIZE 256: {:b}", cr_value, chmod, datatype, keysize);

    // Check that the peripheral is ready (not busy)
    if aes.aes_sr().read().busy().bit_is_set() {
        info!("WARNING: AES peripheral is busy before data input!");
    } else {
        info!("AES peripheral is ready for data input");
    }

    // If CBC mode selected, write IV to AES_IVRx
    // For ECB this step can be skipped
    // for (i, v) in IV128.iter().enumerate() {
    //     match i {
    //         0 => aes.aes_ivr0().write(|w| unsafe { w.bits(*v) }),
    //         1 => aes.aes_ivr1().write(|w| unsafe { w.bits(*v) }),
    //         2 => aes.aes_ivr2().write(|w| unsafe { w.bits(*v) }),
    //         3 => aes.aes_ivr3().write(|w| unsafe { w.bits(*v) }),
    //         _ => {}
    //     }
    // }

    // Write the key into AES_KEYRx registers (KMOD = 0x0)
    // Key registers: AES_KEYR0 .. AES_KEYR7 for 256-bit key (8 x 32-bit)
    // for (i, &kword) in KEY256.iter().enumerate() {
    //     write_key_word(aes, i, kword);
    // }

    info!("Writing key to registers...");
    for (i, kword) in KEY256.iter().enumerate() {
        if i == 0 {
            aes.aes_keyr0().write(|w| unsafe { w.bits(*kword) });
        } else if i == 1 {
            aes.aes_keyr1().write(|w| unsafe { w.bits(*kword) });
        } else if i == 2 {
            aes.aes_keyr2().write(|w| unsafe { w.bits(*kword) });
        } else if i == 3 {
            aes.aes_keyr3().write(|w| unsafe { w.bits(*kword) });
        } else if i == 4 {
            aes.aes_keyr4().write(|w| unsafe { w.bits(*kword) });
        } else if i == 5 {
            aes.aes_keyr5().write(|w| unsafe { w.bits(*kword) });
        } else if i == 6 {
            aes.aes_keyr6().write(|w| unsafe { w.bits(*kword) });
        } else if i == 7 {
            aes.aes_keyr7().write(|w| unsafe { w.bits(*kword) });
        }
    }
    info!("Key written to AES_KEYRx registers.");
    while aes.aes_sr().read().keyvalid().bit_is_clear() {
        asm::nop();
    }
    info!("AES key is valid."); 

    // Enable AES peripheral by setting EN
    aes.aes_cr().modify(|_, w| w.en().set_bit());

    // Append cleartext data block by block
    gpio.gpioa_bsrr().write(|w| w.br12().set_bit()); //set low
    gpio.gpioa_bsrr().write(|w| w.bs12().set_bit()); // set high

    let mut ciphertext_blocks: [[u32; 4]; PLAINTEXT_BLOCKS.len()] = [[0;4]; PLAINTEXT_BLOCKS.len()];

    // info!("Writing plaintext blocks...");
    for (i, block) in PLAINTEXT_BLOCKS.iter().enumerate() {
        // Write the block into AES_DINR
        for word in block.iter() {
            aes.aes_dinr().write(|w| unsafe { w.bits(*word) });
        }

        // Wait until AES is done with this block
        while aes.aes_sr().read().busy().bit_is_set() {
            cortex_m::asm::nop();
        }

        // Read ciphertext from AES_DOUTR
        ciphertext_blocks[i][0] = aes.aes_doutr().read().bits();
        info!("ciphertext_blocks[0] = {:08x}", ciphertext_blocks[i][0]);
        ciphertext_blocks[i][1] = aes.aes_doutr().read().bits();
        info!("ciphertext_blocks[0] = {:08x}", ciphertext_blocks[i][1]);
        ciphertext_blocks[i][2] = aes.aes_doutr().read().bits();
        info!("ciphertext_blocks[0] = {:08x}", ciphertext_blocks[i][2]);
        ciphertext_blocks[i][3] = aes.aes_doutr().read().bits();
        info!("ciphertext_blocks[0] = {:08x}", ciphertext_blocks[i][3]);

        info!("AES ciphertext block {}: {:08x} {:08x} {:08x} {:08x}",
            i,
            ciphertext_blocks[i][0],
            ciphertext_blocks[i][1],
            ciphertext_blocks[i][2],
            ciphertext_blocks[i][3]
        );
    }

    gpio.gpioa_bsrr().write(|w| w.br12().set_bit()); // PA15 LOW -> end measurement

    for (i, block) in ciphertext_blocks.iter().enumerate() {
        info!("AES ciphertext block {}: {:08x} {:08x} {:08x} {:08x}",
            i, block[0], block[1], block[2], block[3]);
    }

    // Finalize sequence: disable AES peripheral (clear EN)
    aes.aes_cr().modify(|_, w| w.en().clear_bit());

    info!("AES sequence done");
    loop {}
}