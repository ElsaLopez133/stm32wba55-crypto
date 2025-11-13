#![no_std]
#![no_main]

// Reference Manual: file:///C:/Users/elopezpe/OneDrive/Documentos/PhD/micro/stm32eba55cg/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf
// use stm32wba::stm32wba55;
use stm32wba::stm32wba55::{self, gpioa};
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;
use cortex_m::asm;
use defmt::info;
use core::{
    mem::size_of,
    ptr::{read_volatile, write_volatile},
};

const MODE: u8 = 0x20;
const BASE: usize = 0x520C_2000;
const PKA_RAM_OFFSET: usize = 0x400; 
const RAM_BASE: usize = BASE + PKA_RAM_OFFSET;
const RAM_NUM_DW: usize = 667;

// PKA RAM locations for ECC multiplication
const PRIME_LENGTH_OFFSET: usize = BASE + 0x400;
const MODULUS_LENGTH_OFFSET: usize = BASE + 0x408;
const COEF_A_SIGN_OFFSET: usize = BASE + 0x410;
const COEF_A_OFFSET: usize = BASE + 0x418;
const COEF_B_OFFSET: usize = BASE + 0x520;
const MODULUS_OFFSET: usize = BASE + 0x1088;
const SCALAR_OFFSET: usize = BASE + 0x12A0;
const POINT_X_OFFSET: usize = BASE + 0x578;
const POINT_Y_OFFSET: usize = BASE + 0x470;
const PRIME_OFFSET: usize = BASE + 0xF88;

const RESULT_X_OFFSET: usize = BASE + 0x578;
const RESULT_Y_OFFSET: usize = BASE + 0x5D0;
const RESULT_ERROR_OFFSET: usize = BASE + 0x680;

const A_SIGN: u32 = 0x1;
const A: [u32; 8] = [
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 
    0x00000000, 0x00000000, 0x00000000, 0x00000003,
];
const N: [u32; 8] = [
    0xffffffff, 0x00000001, 0x00000000, 0x00000000, 
    0x00000000, 0xffffffff, 0xffffffff, 0xffffffff,
];

const B: [u32; 8] = [
    0x5ac635d8, 0xaa3a93e7, 0xb3ebbd55, 0x769886bc,
    0x651d06b0, 0xcc53b0f6, 0x3bce3c3e, 0x27d2604b
];

const BASE_POINT_X: [u32; 8] = [
    0x6b17d1f2, 0xe12c4247, 0xf8bce6e5, 0x63a440f2, 
    0x77037d81, 0x2deb33a0, 0xf4a13945, 0xd898c296,
];

const BASE_POINT_Y: [u32; 8] = [
    0x4fe342e2, 0xfe1a7f9b, 0x8ee7eb4a, 0x7c0f9e16, 
    0x2bce3357, 0x6b315ece, 0xcbb64068, 0x37bf51f5,
];

const PRIME_ORDER: [u32; 8] = [
    0xffffffff, 0x00000000, 0xffffffff, 0xffffffff, 
    0xbce6faad, 0xa7179e84, 0xf3b9cac2, 0xfc632551,
];

const X2: [u32; 8] = [
    0x7CF27B18, 0x8D034F7E, 0x8A523803, 0x04B51AC3, 
    0xC08969E2, 0x77F21B35, 0xA60B48FC, 0x47669978,
];

const Y2: [u32; 8] = [
    0x07775510, 0xDB8ED040, 0x293D9AC6, 0x9F7430DB, 
    0xBA7DADE6, 0x3CE98229, 0x9E04B79D, 0x227873D1,
];

const X3: [u32; 8] = [
    0x5ECBE4D1, 0xA6330A44, 0xC8F7EF95, 0x1D4BF165, 
    0xE6C6B721, 0xEFADA985, 0xFB41661B, 0xC6E7FD6C,
];

const Y3: [u32; 8] = [
    0x8734640C, 0x4998FF7E, 0x374B06CE, 0x1A64A2EC, 
    0xD82AB036, 0x384FB83D, 0x9A79B127, 0xA27D5032,
];

const X4: [u32; 8] = [
    0xE2534A35, 0x32D08FBB, 0xA02DDE65, 0x9EE62BD0, 
    0x031FE2DB, 0x785596EF, 0x50930244, 0x6B030852,
];

const Y4: [u32; 8] = [
    0xE0F1575A, 0x4C633CC7, 0x19DFEE5F, 0xDA862D76, 
    0x4EFC96C3, 0xF30EE005, 0x5C42C23F, 0x184ED8C6,
];

const X5: [u32; 8] = [
    0x51590B7A, 0x515140D2, 0xD784C856, 0x08668FDF, 
    0xEF8C82FD, 0x1F5BE524, 0x21554A0D, 0xC3D033ED,
];

const Y5: [u32; 8] = [
    0xE0C17DA8, 0x904A727D, 0x8AE1BF36, 0xBF8A7926, 
    0x0D012F00, 0xD4D80888, 0xD1D0BB44, 0xFDA16DA4,
];


const SCALAR: [u32; 8] = [0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2];

// const R2MODN: [u32; 8] = [
//     0x00000002, 0x00000000, 0xFFFFFFFA, 0x00000004, 
//     0xFFFFFFFB, 0xFFFFFFFF, 0x00000008, 0xFFFFFFFC
// ];

const OPERAND_LENGTH: u32 = 8 * 32;
const WORD_LENGTH: usize = (OPERAND_LENGTH as usize)/32;   


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
    let gpio = &p.GPIOA;

    // Enable HSI as a stable clock source
    clock.rcc_cr().modify(|_, w| w
    .hseon().set_bit()
    );
    while clock.rcc_cr().read().hserdy().bit_is_clear() {
        asm::nop();
    }

    // Enable RNG clock. Select the source clock. Select the AHB clock
    clock.rcc_ccipr2().write(|w| w.rngsel().b_0x2());
    clock.rcc_ahb2enr().modify(|_, w| {
        w.rngen().set_bit();
        w.gpioaen().set_bit()
    });
    while clock.rcc_ahb2enr().read().rngen().bit_is_clear() {
        asm::nop();
    }
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


    // Configure RNG
    // To configure, CONDRST bit is set to 1 in the same access and CONFIGLOCK remains at 0
    rng.rng_cr().write(|w| w
        .rngen().clear_bit()
        .condrst().set_bit()
        .configlock().clear_bit() 
        .nistc().clear_bit()   
        .ced().clear_bit() 
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
    info!("RNG enabled successfully");

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
    info!("PKA initialized successfully!");

    // Clear any previous error flags
    pka.pka_clrfr().write(|w| w
        .addrerrfc().set_bit()
        .ramerrfc().set_bit()
        .procendfc().set_bit()
    );


    // Write the values - using 32-bit words
    zero_ram();
    write_ram(MODULUS_LENGTH_OFFSET, &[OPERAND_LENGTH]);
    write_ram(PRIME_LENGTH_OFFSET, &[OPERAND_LENGTH]);
    write_ram(COEF_A_SIGN_OFFSET, &[A_SIGN]);

    write_ram(COEF_A_OFFSET, &A);
    write_ram(COEF_B_OFFSET, &B);
    write_ram(MODULUS_OFFSET, &N);
    write_ram(PRIME_OFFSET, &PRIME_ORDER);
    write_ram(POINT_X_OFFSET, &BASE_POINT_X);
    write_ram(POINT_Y_OFFSET, &BASE_POINT_Y);
    write_ram(SCALAR_OFFSET, &SCALAR);

    // Configure PKA operation mode and start
    info!("Starting PKA operation...");
    gpio.gpioa_bsrr().write(|w| w.bs12().set_bit()); // set high

    pka.pka_cr().modify(|_, w| w
        .mode().bits(MODE)
        .start().set_bit()
    );

    // Wait for processing to complete - PROCENDF is 1 when done
    info!("Waiting for operation to complete...");
    while pka.pka_sr().read().procendf().bit_is_clear() {
        asm::nop();
    }
    info!("Operation complete!");
    gpio.gpioa_bsrr().write(|w| w.br12().set_bit());

    // Read the result
    let mut result = [0u32; 1];
    read_ram(RESULT_ERROR_OFFSET, &mut result);
    if result[0] == 0xD60D {
        info!("No errors: {:#X}", result[0]);
        let mut result_x = [0u32; 8];
        let mut result_y = [0u32; 8];
        read_ram(RESULT_X_OFFSET, &mut result_x);
        read_ram(RESULT_Y_OFFSET, &mut result_y);
        info!("POINT (X, Y): ({:#X}, {:#X})", result_x, result_y);
        assert!(result_x == X2);
        assert!(result_y == Y2);
    }
    if result[0] == 0xCBC9 {
        info!("Error in computation: {:#X}", result);
    }
    
    // Clear the completion flag
    pka.pka_clrfr().write(|w| w.procendfc().set_bit());

    loop {}
}