#![no_std]
#![no_main]

// Reference Manual: file:///C:/Users/elopezpe/OneDrive/Documentos/PhD/micro/stm32eba55cg/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf
// use stm32wba::stm32wba55;
use core::{
    mem::size_of,
    ptr::{read_volatile, write_volatile},
};
use cortex_m::asm;
use cortex_m_rt::entry;
use defmt::info;
use stm32wba::stm32wba55::{self, pka::pka_cr::MODE};
use {defmt_rtt as _, panic_probe as _};

#[entry]
fn main() -> ! {
    let p = stm32wba55::Peripherals::take().unwrap();
    let pka = p.PKA;
    let rcc = &p.RCC;
    let rng = &p.RNG;

    let mut pka = Pka::new(pka, rcc, rng);
    info!("PKA Initialized");

    let curve = curve::NIST_P256;
    let mut result: [u32; 8] = [0; 8];

    // Perform ECDSA Signing using PKA
    match pka.modular_subtraction(&curve, &mut result) {
        Ok(_) => {}
        Err(e) => {
            info!("Error during Subtraction");
        }
    }
    info!(
        "{:#X} - {:#X}  = {:#X}",
        curve.operand_a, curve.operand_b, result
    );

    loop {
        asm::nop();
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    Address,
    Ram,
    Mode { mode: u8 },
    Unknown { bits: u32 },
    Busy,
}

impl Error {
    const fn from_raw(raw: u32) -> Result<(), Error> {
        match raw {
            0 => Ok(()),
            _ => Err(Error::Unknown { bits: raw }),
        }
    }

    const fn mode(mode: u8) -> Result<(), Error> {
        Err(Error::Mode { mode })
    }
}

/// PKA operation codes.
#[derive(Debug)]
#[repr(u8)]
#[allow(dead_code)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum PkaOpcode {
    /// Montgomery parameter computation then modular exponentiation.
    MontgomeryParameterExponentiation = 0b000000,
    /// Montgomery parameter computation only.
    MontgomeryParameter = 0b000001,
    /// Modular exponentiation only (Montgomery parameter must be loaded first).
    ModularExponentiation = 0b000010,
    /// Montgomery parameter computation then ECC scalar multiplication.
    MontgomeryParameterEcc = 0b100000,
    /// ECC scalar multiplication only (Montgomery parameter must be loaded first).
    EccScalar = 0b100010,
    /// ECC complete addition.
    EccAddition = 0b100011,
    /// ECC complete addition.
    EccLadder = 0b100111,
    /// ECC complete addition.
    EccProjectiveAffine = 0b101111,
    /// ECDSA signing.
    EcdsaSign = 0b100100,
    /// ECDSA verification.
    EcdsaVerify = 0b100110,
    /// Point on elliptic curve Fp check.
    Point = 0b101000,
    /// RSA CRT exponentiation.
    RsaCrt = 0b000111,
    /// Modular inversion.
    ModularInversion = 0b001000,
    /// Arithmetic addition.
    ArithmeticAdd = 0b001001,
    /// Arithmetic subtraction.
    ArithmeticSub = 0b001010,
    /// Arithmetic multiplication.
    ArithmeticMul = 0b001011,
    /// Arithmetic comparison.
    ArithmeticCmp = 0b001100,
    /// Modular reduction.
    ModularRed = 0b001101,
    /// Modular addition.
    ModularAdd = 0b001110,
    /// Modular subtraction.
    ModularSub = 0b001111,
    /// Montgomery multiplication.
    MontgomeryMul = 0b010000,
}

impl From<PkaOpcode> for u8 {
    fn from(x: PkaOpcode) -> Self {
        x as u8
    }
}

const BASE: usize = 0x520C_2000;
const PKA_RAM_OFFSET: usize = 0x400;
const RAM_BASE: usize = BASE + PKA_RAM_OFFSET;
const RAM_NUM_DW: usize = 667;

// PKA RAM locations
const OPERAND_LENGTH_OFFSET: usize = BASE + 0x408;
const OPERAND_A_OFFSET: usize = BASE + 0xA50;
const OPERAND_B_OFFSET: usize = BASE + 0xC68;
const MODULUS_OFFSET: usize = BASE + 0x1088;
const RESULT_OFFSET: usize = BASE + 0xE78;

/// PKA driver.
#[derive(Debug)]
pub struct Pka {
    pka: stm32wba55::PKA,
}

impl Pka {
    pub fn new(pka: stm32wba55::PKA, rcc: &stm32wba55::RCC, rng: &stm32wba55::RNG) -> Self {
        // Enable HSE (External High-Speed Clock) as a stable clock source
        rcc.rcc_cr().modify(|_, w| w.hseon().set_bit());
        while rcc.rcc_cr().read().hserdy().bit_is_clear() {
            asm::nop();
        }

        // Configure RNG clock
        rcc.rcc_ccipr2().write(|w| w.rngsel().b_0x2());

        // Enable RNG clock on AHB2
        rcc.rcc_ahb2enr().modify(|_, w| w.rngen().set_bit());
        while rcc.rcc_ahb2enr().read().rngen().bit_is_clear() {
            asm::nop();
        }

        // Configure RNG
        rng.rng_cr().write(|w| {
            w.rngen()
                .clear_bit()
                .condrst()
                .set_bit()
                .configlock()
                .clear_bit()
                .nistc()
                .clear_bit()
                .ced()
                .clear_bit()
        });

        // Clear CONDRST while keeping RNGEN disabled
        rng.rng_cr().modify(|_, w| w.condrst().clear_bit());

        // Enable RNG with interrupts
        rng.rng_cr()
            .modify(|_, w| w.rngen().set_bit().ie().set_bit());

        while rng.rng_sr().read().drdy().bit_is_clear() {
            asm::nop();
        }

        // Enable PKA peripheral clock
        rcc.rcc_ahb2enr().modify(|_, w| w.pkaen().set_bit());

        // Reset PKA before enabling (sometimes helps with initialization)
        pka.pka_cr().modify(|_, w| w.en().clear_bit());
        for _ in 0..10 {
            asm::nop();
        }

        // Enable PKA peripheral
        pka.pka_cr().modify(|_, w| w.en().set_bit());

        // Wait for PKA to initialize
        while pka.pka_sr().read().initok().bit_is_clear() {
            asm::nop();
        }
        // info!("PKA initialized successfully!");

        Self { pka }
    }

    /// Returns `true` if the PKA is enabled.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.pka.pka_cr().read().en().bit_is_set()
    }

    #[inline]
    fn clear_all_flags(&mut self) {
        self.pka.pka_clrfr().write(|w| {
            w.addrerrfc().set_bit();
            w.ramerrfc().set_bit();
            w.procendfc().set_bit()
        });
    }

    fn zero_ram(&mut self) {
        (0..RAM_NUM_DW)
            .into_iter()
            .for_each(|dw| unsafe { write_volatile((dw * 4 + RAM_BASE) as *mut u32, 0) });
    }

    unsafe fn write_ram(&mut self, offset: usize, buf: &[u32]) {
        debug_assert_eq!(offset % 4, 0);
        debug_assert!(offset + buf.len() * size_of::<u32>() < 0x520C_33FF);
        buf.iter().rev().enumerate().for_each(|(idx, &dw)| {
            write_volatile((offset + idx * size_of::<u32>()) as *mut u32, dw)
        });
    }

    unsafe fn read_ram(&mut self, offset: usize, buf: &mut [u32]) {
        debug_assert_eq!(offset % 4, 0);
        debug_assert!(offset + buf.len() * size_of::<u32>() < 0x520C_33FF);
        buf.iter_mut().rev().enumerate().for_each(|(idx, dw)| {
            *dw = read_volatile((offset + idx * size_of::<u32>()) as *const u32);
        });
    }

    #[inline]
    unsafe fn start_process(&mut self, opcode: PkaOpcode) {
        self.pka.pka_cr().write(|w| {
            w.addrerrie().set_bit();
            w.ramerrie().set_bit();
            w.procendie().set_bit();
            w.mode().bits(opcode as u8);
            w.start().set_bit();
            w.en().set_bit()
        });
    }

    pub fn modular_subtraction<
        const MODULUS_SIZE: usize,
        const OPERAND_SIZE: usize,
        const PRIME_ORDER_SIZE: usize,
    >(
        &mut self,
        curve: &EllipticCurve<MODULUS_SIZE, PRIME_ORDER_SIZE, OPERAND_SIZE>,
        result: &mut [u32; OPERAND_SIZE],
    ) -> Result<(), Error> {
        self.clear_all_flags();
        self.modular_subtraction_start(curve)?;
        self.modular_subtraction_result(result)
    }

    pub fn modular_subtraction_start<
        const MODULUS_SIZE: usize,
        const OPERAND_SIZE: usize,
        const PRIME_ORDER_SIZE: usize,
    >(
        &mut self,
        curve: &EllipticCurve<MODULUS_SIZE, PRIME_ORDER_SIZE, OPERAND_SIZE>,
    ) -> Result<(), Error> {
        self.zero_ram();
        let operand_length: u32 = (OPERAND_SIZE * size_of::<u32>() * 8) as u32;

        unsafe {
            write_volatile(OPERAND_LENGTH_OFFSET as *mut u32, operand_length);
            self.write_ram(OPERAND_A_OFFSET, &curve.operand_a);
            self.write_ram(OPERAND_B_OFFSET, &curve.operand_b);
            self.write_ram(MODULUS_OFFSET, &curve.modulus);
        }

        let sr = self.pka.pka_sr().read();
        if sr.addrerrf().bit_is_set() {
            self.clear_all_flags();
            Err(Error::Address)
        } else if sr.ramerrf().bit_is_set() {
            self.clear_all_flags();
            Err(Error::Ram)
        } else {
            unsafe {
                self.start_process(PkaOpcode::ModularSub);
            }
            Ok(())
        }
    }

    pub fn modular_subtraction_result<const OPERAND_SIZE: usize>(
        &mut self,
        result: &mut [u32; OPERAND_SIZE],
    ) -> Result<(), Error> {
        let mode = self.pka.pka_cr().read().mode();
        if !mode.is_b_0x_f() {
            return Error::mode(mode.bits());
        }
        let sr = self.pka.pka_sr().read();
        if sr.addrerrf().bit_is_set() {
            self.clear_all_flags();
            return Err(Error::Address);
        } else if sr.ramerrf().bit_is_set() {
            self.clear_all_flags();
            return Err(Error::Ram);
        } else if sr.procendf().bit_is_clear() {
            return Err(Error::Busy);
        } else {
            self.clear_all_flags();

            unsafe {
                self.read_ram(RESULT_OFFSET, result);
            }
        }
        Ok(())
    }
}

#[repr(u32)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Sign {
    Pos = 0,
    Neg = 1,
}

impl From<Sign> for u32 {
    fn from(s: Sign) -> Self {
        s as u32
    }
}

/// Elliptic curve.
#[derive(Debug, PartialEq, Eq)]
pub struct EllipticCurve<
    const MODULUS_SIZE: usize,
    const PRIME_ORDER_SIZE: usize,
    const OPERAND_SIZE: usize,
> {
    /// Curve coefficient a sign.
    ///
    /// **Note:** 0 for positive, 1 for negative.
    pub coef_sign: Sign,
    /// Curve coefficient |a|.
    ///
    /// **Note:** Absolute value, |a| < p.
    pub coef: [u32; MODULUS_SIZE],
    /// Curve modulus value p.
    ///
    /// **Note:** Odd integer prime, 0 < p < 2<sup>640</sup>
    pub modulus: [u32; MODULUS_SIZE],
    /// Curve base point G coordinate x.
    ///
    /// **Note:** x < p
    pub base_point_x: [u32; MODULUS_SIZE],
    /// Curve base point G coordinate y.
    ///
    /// **Note:** y < p
    pub base_point_y: [u32; MODULUS_SIZE],
    /// Curve prime order n.
    ///
    /// **Note:** Integer prime.
    pub prime_order: [u32; PRIME_ORDER_SIZE],
    // Operand examples
    pub operand_a: [u32; OPERAND_SIZE],
    pub operand_b: [u32; OPERAND_SIZE],
}

/// Pre-defined elliptic curves.
pub mod curve {
    use super::{
        EllipticCurve,
        Sign::{Neg, Pos},
    };

    /// nist P-256
    pub const NIST_P256: EllipticCurve<8, 8, 8> = EllipticCurve {
        coef_sign: Neg,
        coef: [
            0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
            0x00000003,
        ],
        modulus: [
            0xffffffff, 0x00000001, 0x00000000, 0x00000000, 0x00000000, 0xffffffff, 0xffffffff,
            0xffffffff,
        ],
        base_point_x: [
            0x6b17d1f2, 0xe12c4247, 0xf8bce6e5, 0x63a440f2, 0x77037d81, 0x2deb33a0, 0xf4a13945,
            0xd898c296,
        ],
        base_point_y: [
            0x4fe342e2, 0xfe1a7f9b, 0x8ee7eb4a, 0x7c0f9e16, 0x2bce3357, 0x6b315ece, 0xcbb64068,
            0x37bf51f5,
        ],
        prime_order: [
            0xffffffff, 0x00000000, 0xffffffff, 0xffffffff, 0xbce6faad, 0xa7179e84, 0xf3b9cac2,
            0xfc632551,
        ],
        operand_a: [
            0xffffffff, 0x00000001, 0x00000000, 0x00000000, 0x00000000, 0xffffffff, 0xffffffff,
            0xfffffffe,
        ],
        operand_b: [
            0xffffffff, 0x00000001, 0x00000000, 0x00000000, 0x00000000, 0xffffffff, 0xffffffff,
            0xfffffff0,
        ],
    };
}
