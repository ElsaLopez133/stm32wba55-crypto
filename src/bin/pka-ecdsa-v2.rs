#![no_std]
#![no_main]

// Reference Manual: file:///C:/Users/elopezpe/OneDrive/Documentos/PhD/micro/stm32eba55cg/rm0493-multiprotocol-wireless-bluetooth-low-energy-and-ieee802154-stm32wba5xxx-arm-based-32-bit-mcus-stmicroelectronics-en.pdf
// use stm32wba::stm32wba55;
use stm32wba::stm32wba55::{self,pka::pka_cr::MODE};
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;
use cortex_m::asm;
use defmt::info;
use core::{
    mem::size_of,
    ptr::{read_volatile, write_volatile},
};

#[entry]
fn main() -> ! {
    let p = stm32wba55::Peripherals::take().unwrap();
    let pka = p.PKA;
    let rcc = &p.RCC;
    let rng = &p.RNG;

    let mut pka = Pka::new(pka, rcc, rng);
    info!("PKA Initialized");

    let curve = curve::NIST_P256;
    let nonce: [u32; 8] = [0; 8];
    let priv_key: [u32; 8] = PRIV_KEY;
    let hash: [u32; 8] = [0; 8];
    let mut r_sign: [u32; 8] = [0; 8];
    let mut s_sign: [u32; 8] = [0; 8];
    
    // Perform ECDSA Signing using PKA
    match pka.ecdsa_sign(&curve, &nonce, &priv_key, &hash, &mut r_sign, &mut s_sign) {
        Ok(_) => {},
        Err(e) => {
            info!("Error during ECDSA signing: {:?}", e);
        }
    }
    
    info!("ECDSA Signature r: {:#X}", r_sign);
    info!("ECDSA Signature s: {:#X}", s_sign);

    loop {
        asm::nop();
    }
}

/// Errors from an ECDSA signing operation.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EcdsaSignError {
    /// Address access is out of range (unmapped address).
    Address,
    /// An AHB access to the PKA RAM occurred while the PKA core was computing
    /// and using its internal RAM.
    Ram,
    /// Signature part R is equal to 0.
    Rzero,
    /// Signature part S is equal to 0.
    Szero,
    /// PKA mode does not match the expected mode.
    Mode {
        /// Actual mode bits
        mode: u8,
    },
    /// Unknown result code.
    Unknown {
        /// Unknown result code bits.
        bits: u32,
    },
    NonComplete,
}

impl EcdsaSignError {
    const fn from_raw(raw: u32) -> Result<(), EcdsaSignError> {
        match raw {
            0 => Ok(()),
            1 => Err(EcdsaSignError::Rzero),
            2 => Err(EcdsaSignError::Szero),
            _ => Err(EcdsaSignError::Unknown { bits: raw }),
        }
    }

    const fn mode(mode: u8) -> Result<(), EcdsaSignError> {
        Err(EcdsaSignError::Mode { mode })
    }
}

/// Errors from an ECDSA verify operation.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EcdsaVerifyError {
    /// Address access is out of range (unmapped address).
    Address,
    /// An AHB access to the PKA RAM occurred while the PKA core was computing
    /// and using its internal RAM
    Ram,
    /// Invalid signature.
    Invalid,
    /// PKA mode does not match the expected mode.
    Mode {
        /// Actual mode bits
        mode: u8,
    },
    NonComplete,
}

impl EcdsaVerifyError {
    const fn from_raw(raw: u32) -> Result<(), EcdsaVerifyError> {
        match raw {
            0 => Ok(()),
            _ => Err(EcdsaVerifyError::Invalid),
        }
    }

    const fn mode(mode: u8) -> Result<(), EcdsaVerifyError> {
        Err(EcdsaVerifyError::Mode { mode })
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

// ECDSA sign input addresses
const ECDSA_SIGN_N_LEN: usize = BASE + 0x400;
const ECDSA_SIGN_P_LEN: usize = BASE + 0x408;
const ECDSA_SIGN_A_SIGN: usize = BASE + 0x410;
const ECDSA_SIGN_A: usize = BASE + 0x418;
const ECDSA_SIGN_B: usize = BASE + 0x520;
const ECDSA_SIGN_P: usize = BASE + 0x1088;
const ECDSA_SIGN_K: usize = BASE + 0x12A0;
const ECDSA_SIGN_X: usize = BASE + 0x578;
const ECDSA_SIGN_Y: usize = BASE + 0x470;
const ECDSA_SIGN_Z: usize = BASE + 0xFE8;
const ECDSA_SIGN_D: usize = BASE + 0xF28;
const ECDSA_SIGN_N: usize = BASE + 0xF88;

// ECDSA sign output addresses
const ECDSA_SIGN_OUT_R: usize = BASE + 0x730;
const ECDSA_SIGN_OUT_S: usize = BASE + 0x788;
const ECDSA_SIGN_OUT_RESULT: usize = BASE + 0xFE0;

// ECDSA verify input addresses
const ECDSA_VERIFY_N_LEN: usize = BASE + 0x408;
const ECDSA_VERIFY_P_LEN: usize = BASE + 0x4C8;
const ECDSA_VERIFY_A_SIGN: usize = BASE + 0x468;
const ECDSA_VERIFY_A: usize = BASE + 0x470;
const ECDSA_VERIFY_P: usize = BASE + 0x4D0;
const ECDSA_VERIFY_X: usize = BASE + 0x678;
const ECDSA_VERIFY_Y: usize = BASE + 0x6D0;
const ECDSA_VERIFY_XQ: usize = BASE + 0x12F8;
const ECDSA_VERIFY_YQ: usize = BASE + 0x1350;
const ECDSA_VERIFY_R: usize = BASE + 0x10E0;
const ECDSA_VERIFY_S: usize = BASE + 0xC68;
const ECDSA_VERIFY_Z: usize = BASE + 0x13A8;
const ECDSA_VERIFY_N: usize = BASE + 0x1088;

// ECDSA verify output addresses
const ECDSA_VERIFY_OUT: usize = BASE + 0x5B0;
const ECDSA_VERIFY_SIGN_OUT_R: usize = BASE + 0x578;

const PRIV_KEY: [u32; 8] = [
     0x49ac8727, 0xcee87484, 0xfe6dfda5, 0x10238ad4, 
     0x11ace8fe, 0x593a8cb7, 0x0492d659, 0xdb81802a,
];

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
        rng.rng_cr().write(|w| w
            .rngen().clear_bit()
            .condrst().set_bit()
            .configlock().clear_bit() 
            .nistc().clear_bit()   
            .ced().clear_bit() 
        );

        // Clear CONDRST while keeping RNGEN disabled
        rng.rng_cr().modify(|_, w| w.condrst().clear_bit());

        // Enable RNG with interrupts
        rng.rng_cr().modify(|_, w| w
            .rngen().set_bit()
            .ie().set_bit()
        );
        
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

    // /// Free the PKA peripheral from the driver.
    // pub fn free(self) -> stm32wba55::PKA {
    //     self.pka
    // }

    // pub unsafe fn steal() -> Self {
    //     Self {
    //         pka: stm32wba55::Peripherals::steal().PKA,
    //     }
    // }

    // /// Disable the PKA clock.
    // pub unsafe fn disable_clock(rcc: &mut stm32wba55::RCC) {
    //     rcc.rcc_ahb2enr().modify(|_, w| w.pkaen().clear_bit());
    // }

    // /// Enable the PKA clock.
    // pub fn enable_clock(rcc: &mut stm32wba55::RCC) {
    //     rcc.rcc_ahb2enr().modify(|_, w| w.pkaen().set_bit());
    //     let _ = rcc.rcc_ahb2enr().read();
    // }

    // pub unsafe fn pulse_reset(rcc: &stm32wba55::RCC) {
    //     // Pulse reset by setting and clearing the reset bit
    //     rcc.rcc_ahb2rstr().modify(|_, w| w.pkarst().set_bit());
    //     rcc.rcc_ahb2rstr().modify(|_, w| w.pkarst().clear_bit());
    // }

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
    fn start_process(&mut self, mode: MODE) {
        self.pka.pka_cr().write(|w| {
            w.addrerrie().set_bit();
            w.ramerrie().set_bit();
            w.procendie().set_bit();
            w.mode().variant(mode);
            w.start().set_bit();
            w.en().set_bit()
        });

        // info!("Mode 0x24: {:#X}", self.pka.pka_cr().read().mode().is_b_0x24());
    }

    pub fn ecdsa_sign<const MODULUS_SIZE: usize, const PRIME_ORDER_SIZE: usize>(
        &mut self,
        curve: &EllipticCurve<MODULUS_SIZE, PRIME_ORDER_SIZE>,
        nonce: &[u32; PRIME_ORDER_SIZE],
        priv_key: &[u32; PRIME_ORDER_SIZE],
        hash: &[u32; PRIME_ORDER_SIZE],
        r_sign: &mut [u32; MODULUS_SIZE],
        s_sign: &mut [u32; MODULUS_SIZE],
    ) -> Result<(), EcdsaSignError> {
        self.clear_all_flags();
        self.ecdsa_sign_start(curve, nonce, priv_key, hash)?;
        self.ecdsa_sign_result(r_sign, s_sign)
    }

    pub fn ecdsa_sign_start<const MODULUS_SIZE: usize, const PRIME_ORDER_SIZE: usize>(
        &mut self,
        curve: &EllipticCurve<MODULUS_SIZE, PRIME_ORDER_SIZE>,
        nonce: &[u32; PRIME_ORDER_SIZE],
        priv_key: &[u32; PRIME_ORDER_SIZE],
        hash: &[u32; PRIME_ORDER_SIZE],
    ) -> Result<(), EcdsaSignError> {
        self.zero_ram();
        let n_length: u32 = (PRIME_ORDER_SIZE * size_of::<u32>() * 8) as u32;
        let p_length: u32 = (MODULUS_SIZE * size_of::<u32>() * 8) as u32;

        // info!("n_lenght: {:?}   p_length: {:?}", n_length, p_length);
        // info!("PRIME_ORDER_SIZE: {:?}   MODULUS_SIZE: {:?}", PRIME_ORDER_SIZE, MODULUS_SIZE);

        unsafe {
            write_volatile(ECDSA_SIGN_N_LEN as *mut u32, n_length);
            write_volatile(ECDSA_SIGN_P_LEN as *mut u32, p_length);
            write_volatile(ECDSA_SIGN_A_SIGN as *mut u32, curve.coef_sign.into());
            self.write_ram(ECDSA_SIGN_A, &curve.coef);
            self.write_ram(ECDSA_SIGN_P, &curve.modulus);
            self.write_ram(ECDSA_SIGN_K, nonce);
            self.write_ram(ECDSA_SIGN_X, &curve.base_point_x);
            self.write_ram(ECDSA_SIGN_Y, &curve.base_point_y);
            self.write_ram(ECDSA_SIGN_Z, hash);
            self.write_ram(ECDSA_SIGN_D, priv_key);
            self.write_ram(ECDSA_SIGN_N, &curve.prime_order);
        }

        let sr = self.pka.pka_sr().read();
        if sr.addrerrf().bit_is_set() {
            self.clear_all_flags();
            Err(EcdsaSignError::Address)
        } else if sr.ramerrf().bit_is_set() {
            self.clear_all_flags();
            Err(EcdsaSignError::Ram)
        } else {
            self.start_process(MODE::B0x24);
            // Wait for processing to complete - PROCENDF is 1 when done
            info!("Waiting for operation to complete...");
            while sr.procendf().bit_is_clear() {
                asm::nop();
            }
            Ok(())
        }
    }

    pub fn ecdsa_sign_result<const MODULUS_SIZE: usize>(
        &mut self,
        r_sign: &mut [u32; MODULUS_SIZE],
        s_sign: &mut [u32; MODULUS_SIZE],
    ) -> Result<(), EcdsaSignError> {
        let mode =self.pka.pka_cr().read().mode();
        if !mode.is_b_0x24() {
            return EcdsaSignError::mode(mode.bits());
        }
        let sr = self.pka.pka_sr().read();
        if sr.addrerrf().bit_is_set() {
            self.clear_all_flags();
            Err(EcdsaSignError::Address)
        } else if sr.ramerrf().bit_is_set() {
            self.clear_all_flags();
            Err(EcdsaSignError::Ram)
        } else if sr.procendf().bit_is_clear() {
            Err(EcdsaSignError::NonComplete)
        } else {
            self.clear_all_flags();

            unsafe {
                self.read_ram(ECDSA_SIGN_OUT_R, r_sign);
                self.read_ram(ECDSA_SIGN_OUT_S, s_sign);
            }

            let result: u32 = unsafe { read_volatile(ECDSA_SIGN_OUT_RESULT as *const u32) };
            if result != 0 {
                self.zero_ram();
            }
            EcdsaSignError::from_raw(result)
        }
    }

    pub fn ecdsa_verify<const MODULUS_SIZE: usize, const PRIME_ORDER_SIZE: usize>(
        &mut self,
        curve: &EllipticCurve<MODULUS_SIZE, PRIME_ORDER_SIZE>,
        sig: &EcdsaSignature<MODULUS_SIZE>,
        pub_key: &EcdsaPublicKey<MODULUS_SIZE>,
        hash: &[u32; PRIME_ORDER_SIZE],
    ) -> Result<(), EcdsaVerifyError> {
        self.ecdsa_verify_start(curve, sig, pub_key, hash)?;
        self.ecdsa_verify_result()
    }

    pub fn ecdsa_verify_start<const MODULUS_SIZE: usize, const PRIME_ORDER_SIZE: usize>(
        &mut self,
        curve: &EllipticCurve<MODULUS_SIZE, PRIME_ORDER_SIZE>,
        sig: &EcdsaSignature<MODULUS_SIZE>,
        pub_key: &EcdsaPublicKey<MODULUS_SIZE>,
        hash: &[u32; PRIME_ORDER_SIZE],
    ) -> Result<(), EcdsaVerifyError> {
        self.zero_ram();
        let n_length: u32 = (PRIME_ORDER_SIZE * size_of::<u32>() * 8) as u32;
        let p_length: u32 = (MODULUS_SIZE * size_of::<u32>() * 8) as u32;

        unsafe {
            write_volatile(ECDSA_VERIFY_N_LEN as *mut u32, n_length);
            write_volatile(ECDSA_VERIFY_P_LEN as *mut u32, p_length);
            write_volatile(ECDSA_VERIFY_A_SIGN as *mut u32, curve.coef_sign.into());
            self.write_ram(ECDSA_VERIFY_A, &curve.coef);
            self.write_ram(ECDSA_VERIFY_P, &curve.modulus);
            self.write_ram(ECDSA_VERIFY_X, &curve.base_point_x);
            self.write_ram(ECDSA_VERIFY_Y, &curve.base_point_y);
            self.write_ram(ECDSA_VERIFY_XQ, pub_key.curve_pt_x);
            self.write_ram(ECDSA_VERIFY_YQ, pub_key.curve_pt_y);
            self.write_ram(ECDSA_VERIFY_R, sig.r_sign);
            self.write_ram(ECDSA_VERIFY_S, sig.s_sign);
            self.write_ram(ECDSA_VERIFY_Z, hash);
            self.write_ram(ECDSA_VERIFY_N, &curve.prime_order);
        }
        let sr = self.pka.pka_sr().read();
        if sr.addrerrf().bit_is_set() {
            self.clear_all_flags();
            Err(EcdsaVerifyError::Address)
        } else if sr.ramerrf().bit_is_set() {
            self.clear_all_flags();
            Err(EcdsaVerifyError::Ram)
        } else {
            self.start_process(MODE::B0x26);
            Ok(())
        }
    }

    pub fn ecdsa_verify_result(&mut self) -> Result<(), EcdsaVerifyError> {
        let mode = self.pka.pka_cr().read().mode();
        if !mode.is_b_0x26() {
            return EcdsaVerifyError::mode(mode.bits());
        }
        let sr = self.pka.pka_sr().read();
        if sr.addrerrf().bit_is_set() {
            self.clear_all_flags();
            Err(EcdsaVerifyError::Address)
        } else if sr.ramerrf().bit_is_set() {
            self.clear_all_flags();
            Err(EcdsaVerifyError::Ram)
        } else if sr.procendf().bit_is_clear() {
            Err(EcdsaVerifyError::NonComplete)
        } else {
            self.clear_all_flags();

            let result: u32 = unsafe { read_volatile(ECDSA_VERIFY_OUT as *const u32) };
            EcdsaVerifyError::from_raw(result)
        }
    }
}

/// Sign bit for ECDSA coefficient signing and verification.
#[repr(u32)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Sign {
    /// Positive.
    Pos = 0,
    /// Negative.
    Neg = 1,
}

impl From<Sign> for u32 {
    fn from(s: Sign) -> Self {
        s as u32
    }
}

/// ECDSA signature.
#[derive(Debug, PartialEq, Eq)]
pub struct EcdsaSignature<'a, const MODULUS_SIZE: usize> {
    /// Signature part r.
    pub r_sign: &'a [u32; MODULUS_SIZE],
    /// Signature part s.
    pub s_sign: &'a [u32; MODULUS_SIZE],
}

#[cfg(feature = "defmt")]
impl<'a, const MODULUS_SIZE: usize> defmt::Format for EcdsaSignature<'a, MODULUS_SIZE> {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "EcdsaSignature {{ r_sign: {}, s_sign: {} }}",
            self.r_sign.as_ref(),
            self.s_sign.as_ref()
        )
    }
}

/// ECDSA public key.
#[derive(Debug, PartialEq, Eq)]
pub struct EcdsaPublicKey<'a, const MODULUS_SIZE: usize> {
    /// Public-key curve point xQ.
    pub curve_pt_x: &'a [u32; MODULUS_SIZE],
    /// Public-key curve point yQ.
    pub curve_pt_y: &'a [u32; MODULUS_SIZE],
}

/// Elliptic curve.
///
/// Used to ECDSA signing and verification.
#[derive(Debug, PartialEq, Eq)]
pub struct EllipticCurve<const MODULUS_SIZE: usize, const PRIME_ORDER_SIZE: usize> {
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
}

/// Pre-defined elliptic curves.
pub mod curve {
    use super::{
        EllipticCurve,
        Sign::{Neg, Pos},
    };

    /// nist P-256
    pub const NIST_P256: EllipticCurve<8, 8> = EllipticCurve {
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
    };
}
