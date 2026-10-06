//! Controls (IEEE 1722.1-2021, 7.2.22 and 7.3.4 to 7.3.6): the CONTROL
//! descriptor, what kind of control it is, the type and units of its
//! values, and the current values SET_CONTROL and GET_CONTROL carry.

use crate::avtp::{read_array, read_u16, read_u32, read_u64};
use crate::descriptor::{DescriptorType, LocalizedStringRef, aem_string};
use crate::error::DecodeError;
use crate::macros::code;

code! {
    /// control_type (Table 7-98): the controls the standard defines, under
    /// its OUI-36 90-e0-f0-0.
    pub struct ControlType(u64) {
        const ENABLE = 0x90e0_f000_0000_0000;
        const IDENTIFY = 0x90e0_f000_0000_0001;
        const MUTE = 0x90e0_f000_0000_0002;
        const INVERT = 0x90e0_f000_0000_0003;
        const GAIN = 0x90e0_f000_0000_0004;
        const ATTENUATE = 0x90e0_f000_0000_0005;
        const DELAY = 0x90e0_f000_0000_0006;
        const SRC_MODE = 0x90e0_f000_0000_0007;
        const SNAPSHOT = 0x90e0_f000_0000_0008;
        const POW_LINE_FREQ = 0x90e0_f000_0000_0009;
        const POWER_STATUS = 0x90e0_f000_0000_000a;
        const FAN_STATUS = 0x90e0_f000_0000_000b;
        const TEMPERATURE = 0x90e0_f000_0000_000c;
        const ALTITUDE = 0x90e0_f000_0000_000d;
        const ABSOLUTE_HUMIDITY = 0x90e0_f000_0000_000e;
        const RELATIVE_HUMIDITY = 0x90e0_f000_0000_000f;
        const ORIENTATION = 0x90e0_f000_0000_0010;
        const VELOCITY = 0x90e0_f000_0000_0011;
        const ACCELERATION = 0x90e0_f000_0000_0012;
        const FILTER_RESPONSE = 0x90e0_f000_0000_0013;
        const BAROMETRIC_PRESSURE = 0x90e0_f000_0000_0014;
        const MANUFACTURER_URL = 0x90e0_f000_0000_0015;
        const ENTITY_URL = 0x90e0_f000_0000_0016;
        const CONFIGURATION_URL = 0x90e0_f000_0000_0017;
        const GENERIC_URL = 0x90e0_f000_0000_0018;
        const FAULT = 0x90e0_f000_0000_0019;
        const CONTROLLER_TARGET_ENTITY = 0x90e0_f000_0000_001a;
        const CONTROLLER_TARGET_OBJECT = 0x90e0_f000_0000_001b;
        const LATENCY_COMPENSATION = 0x90e0_f000_0000_001c;
        const PANPOT = 0x90e0_f000_0000_001d;
        const PHANTOM = 0x90e0_f000_0000_001e;
        const AUDIO_SCALE = 0x90e0_f000_0000_001f;
        const AUDIO_METERS = 0x90e0_f000_0000_0020;
        const AUDIO_SPECTRUM = 0x90e0_f000_0000_0021;
    }
}

code! {
    /// The value_type of a control_value_type (Table 7-121), without its
    /// read only and unknown flags.
    pub struct ValueType(u16) {
        const LINEAR_INT8 = 0x0000;
        const LINEAR_UINT8 = 0x0001;
        const LINEAR_INT16 = 0x0002;
        const LINEAR_UINT16 = 0x0003;
        const LINEAR_INT32 = 0x0004;
        const LINEAR_UINT32 = 0x0005;
        const LINEAR_INT64 = 0x0006;
        const LINEAR_UINT64 = 0x0007;
        const LINEAR_FLOAT = 0x0008;
        const LINEAR_DOUBLE = 0x0009;
        const SELECTOR_INT8 = 0x000a;
        const SELECTOR_UINT8 = 0x000b;
        const SELECTOR_INT16 = 0x000c;
        const SELECTOR_UINT16 = 0x000d;
        const SELECTOR_INT32 = 0x000e;
        const SELECTOR_UINT32 = 0x000f;
        const SELECTOR_INT64 = 0x0010;
        const SELECTOR_UINT64 = 0x0011;
        const SELECTOR_FLOAT = 0x0012;
        const SELECTOR_DOUBLE = 0x0013;
        const SELECTOR_STRING = 0x0014;
        const ARRAY_INT8 = 0x0015;
        const ARRAY_UINT8 = 0x0016;
        const ARRAY_INT16 = 0x0017;
        const ARRAY_UINT16 = 0x0018;
        const ARRAY_INT32 = 0x0019;
        const ARRAY_UINT32 = 0x001a;
        const ARRAY_INT64 = 0x001b;
        const ARRAY_UINT64 = 0x001c;
        const ARRAY_FLOAT = 0x001d;
        const ARRAY_DOUBLE = 0x001e;
        const UTF8 = 0x001f;
        const BODE_PLOT = 0x0020;
        const SMPTE_TIME = 0x0021;
        const SAMPLE_RATE = 0x0022;
        const GPTP_TIME = 0x0023;
        const VENDOR = 0x3ffe;
        const EXPANSION = 0x3fff;
    }
}

/// How a control's values are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Each value with its own range, step, default and unit.
    Linear,
    /// One value from a list of options.
    Selector,
    /// Values sharing one range, step, default and unit.
    Array,
    /// One text.
    Utf8,
    /// A Bode plot, a time, a sample rate or a vendor's blob.
    Other,
}

/// How each value of a numeric control is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scalar {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

/// The scalars in the order each group of value types lists them.
const SCALARS: [Scalar; 10] = [
    Scalar::I8,
    Scalar::U8,
    Scalar::I16,
    Scalar::U16,
    Scalar::I32,
    Scalar::U32,
    Scalar::I64,
    Scalar::U64,
    Scalar::F32,
    Scalar::F64,
];

impl ValueType {
    pub fn shape(self) -> Shape {
        match self.0 {
            0x0000..=0x0009 => Shape::Linear,
            0x000a..=0x0014 => Shape::Selector,
            0x0015..=0x001e => Shape::Array,
            0x001f => Shape::Utf8,
            _ => Shape::Other,
        }
    }

    /// The scalar a numeric control's values are stored as; a string
    /// selector's options are localized string references.
    pub fn scalar(self) -> Option<Scalar> {
        let place = match self.0 {
            0x0000..=0x0009 => self.0,
            0x000a..=0x0013 => self.0 - 0x000a,
            0x0014 => return Some(Scalar::U16),
            0x0015..=0x001e => self.0 - 0x0015,
            _ => return None,
        };
        Some(SCALARS[usize::from(place)])
    }
}

impl Scalar {
    pub const fn size(self) -> usize {
        match self {
            Scalar::I8 | Scalar::U8 => 1,
            Scalar::I16 | Scalar::U16 => 2,
            Scalar::I32 | Scalar::U32 | Scalar::F32 => 4,
            Scalar::I64 | Scalar::U64 | Scalar::F64 => 8,
        }
    }

    /// Whether the scalar holds whole numbers.
    pub const fn whole(self) -> bool {
        !matches!(self, Scalar::F32 | Scalar::F64)
    }

    /// The value at the start of `bytes`, which hold at least its size.
    fn read(self, bytes: &[u8]) -> Number {
        match self {
            Scalar::I8 => Number::Int(i64::from(bytes[0] as i8)),
            Scalar::U8 => Number::Uint(u64::from(bytes[0])),
            Scalar::I16 => Number::Int(i64::from(read_u16(bytes, 0) as i16)),
            Scalar::U16 => Number::Uint(u64::from(read_u16(bytes, 0))),
            Scalar::I32 => Number::Int(i64::from(read_u32(bytes, 0) as i32)),
            Scalar::U32 => Number::Uint(u64::from(read_u32(bytes, 0))),
            Scalar::I64 => Number::Int(read_u64(bytes, 0) as i64),
            Scalar::U64 => Number::Uint(read_u64(bytes, 0)),
            Scalar::F32 => Number::Float(f64::from(f32::from_bits(read_u32(bytes, 0)))),
            Scalar::F64 => Number::Float(f64::from_bits(read_u64(bytes, 0))),
        }
    }

    /// Writes `number` at the start of `out`, which holds at least the
    /// scalar's size, saturating at the scalar's range.
    pub fn write(self, number: Number, out: &mut [u8]) {
        let whole = number.whole();
        let size = self.size();
        match self {
            Scalar::I8 => out[0] = whole.clamp(i8::MIN.into(), i8::MAX.into()) as i8 as u8,
            Scalar::U8 => out[0] = whole.clamp(0, u8::MAX.into()) as u8,
            Scalar::I16 => out[..size].copy_from_slice(
                &(whole.clamp(i16::MIN.into(), i16::MAX.into()) as i16).to_be_bytes(),
            ),
            Scalar::U16 => {
                out[..size].copy_from_slice(&(whole.clamp(0, u16::MAX.into()) as u16).to_be_bytes())
            }
            Scalar::I32 => out[..size].copy_from_slice(
                &(whole.clamp(i32::MIN.into(), i32::MAX.into()) as i32).to_be_bytes(),
            ),
            Scalar::U32 => {
                out[..size].copy_from_slice(&(whole.clamp(0, u32::MAX.into()) as u32).to_be_bytes())
            }
            Scalar::I64 => out[..size].copy_from_slice(
                &(whole.clamp(i64::MIN.into(), i64::MAX.into()) as i64).to_be_bytes(),
            ),
            Scalar::U64 => {
                out[..size].copy_from_slice(&(whole.clamp(0, u64::MAX.into()) as u64).to_be_bytes())
            }
            Scalar::F32 => out[..size].copy_from_slice(&(number.to_f64() as f32).to_be_bytes()),
            Scalar::F64 => out[..size].copy_from_slice(&number.to_f64().to_be_bytes()),
        }
    }
}

/// A control value, as its scalar holds it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    Int(i64),
    Uint(u64),
    Float(f64),
}

impl Number {
    pub fn to_f64(self) -> f64 {
        match self {
            Number::Int(value) => value as f64,
            Number::Uint(value) => value as f64,
            Number::Float(value) => value,
        }
    }

    /// The number as a whole one, wide enough for any scalar; a float
    /// goes towards zero.
    fn whole(self) -> i128 {
        match self {
            Number::Int(value) => value.into(),
            Number::Uint(value) => value.into(),
            Number::Float(value) => value as i128,
        }
    }
}

code! {
    /// A unit's base quantity (Tables 7-74 to 7-97).
    pub struct UnitCode(u8) {
        const UNITLESS = 0x00;
        const COUNT = 0x01;
        const PERCENT = 0x02;
        const FSTOP = 0x03;
        const SECONDS = 0x08;
        const MINUTES = 0x09;
        const HOURS = 0x0a;
        const DAYS = 0x0b;
        const MONTHS = 0x0c;
        const YEARS = 0x0d;
        const SAMPLES = 0x0e;
        const FRAMES = 0x0f;
        const HERTZ = 0x10;
        const SEMITONES = 0x11;
        const CENTS = 0x12;
        const OCTAVES = 0x13;
        const FPS = 0x14;
        const METRES = 0x18;
        const KELVIN = 0x20;
        const GRAMS = 0x28;
        const VOLTS = 0x30;
        const DBV = 0x31;
        const DBU = 0x32;
        const AMPS = 0x38;
        const WATTS = 0x40;
        const DBM = 0x41;
        const DBW = 0x42;
        const PASCALS = 0x48;
        const BITS = 0x50;
        const BYTES = 0x51;
        const KIBIBYTES = 0x52;
        const MEBIBYTES = 0x53;
        const GIBIBYTES = 0x54;
        const TEBIBYTES = 0x55;
        const BITS_PER_SEC = 0x58;
        const BYTES_PER_SEC = 0x59;
        const KIBIBYTES_PER_SEC = 0x5a;
        const MEBIBYTES_PER_SEC = 0x5b;
        const GIBIBYTES_PER_SEC = 0x5c;
        const TEBIBYTES_PER_SEC = 0x5d;
        const CANDELAS = 0x60;
        const JOULES = 0x68;
        const RADIANS = 0x70;
        const NEWTONS = 0x78;
        const OHMS = 0x80;
        const METRES_PER_SEC = 0x88;
        const RADIANS_PER_SEC = 0x89;
        const METRES_PER_SEC_SQUARED = 0x90;
        const RADIANS_PER_SEC_SQUARED = 0x91;
        const TESLAS = 0x98;
        const WEBERS = 0x99;
        const AMPS_PER_METRE = 0x9a;
        const METRES_SQUARED = 0xa0;
        const METRES_CUBED = 0xa8;
        const LITRES = 0xa9;
        const DB = 0xb0;
        const DB_PEAK = 0xb1;
        const DB_RMS = 0xb2;
        const DBFS = 0xb3;
        const DBFS_PEAK = 0xb4;
        const DBFS_RMS = 0xb5;
        const DBTP = 0xb6;
        const DB_SPL_A = 0xb7;
        const DB_Z = 0xb8;
        const DB_SPL_C = 0xb9;
        const DB_SPL = 0xba;
        const LU = 0xbb;
        const LUFS = 0xbc;
        const DB_A = 0xbd;
    }
}

impl UnitCode {
    /// What follows a value in the unit, empty for none.
    pub fn suffix(self) -> &'static str {
        match self {
            UnitCode::PERCENT => "%",
            UnitCode::SECONDS => "s",
            UnitCode::MINUTES => "min",
            UnitCode::HOURS => "h",
            UnitCode::DAYS => "d",
            UnitCode::MONTHS => "months",
            UnitCode::YEARS => "years",
            UnitCode::SAMPLES => "samples",
            UnitCode::FRAMES => "frames",
            UnitCode::HERTZ => "Hz",
            UnitCode::SEMITONES => "semitones",
            UnitCode::CENTS => "cents",
            UnitCode::OCTAVES => "octaves",
            UnitCode::FPS => "fps",
            UnitCode::METRES => "m",
            UnitCode::KELVIN => "K",
            UnitCode::GRAMS => "g",
            UnitCode::VOLTS => "V",
            UnitCode::DBV => "dBV",
            UnitCode::DBU => "dBu",
            UnitCode::AMPS => "A",
            UnitCode::WATTS => "W",
            UnitCode::DBM => "dBm",
            UnitCode::DBW => "dBW",
            UnitCode::PASCALS => "Pa",
            UnitCode::BITS => "b",
            UnitCode::BYTES => "B",
            UnitCode::KIBIBYTES => "KiB",
            UnitCode::MEBIBYTES => "MiB",
            UnitCode::GIBIBYTES => "GiB",
            UnitCode::TEBIBYTES => "TiB",
            UnitCode::BITS_PER_SEC => "b/s",
            UnitCode::BYTES_PER_SEC => "B/s",
            UnitCode::KIBIBYTES_PER_SEC => "KiB/s",
            UnitCode::MEBIBYTES_PER_SEC => "MiB/s",
            UnitCode::GIBIBYTES_PER_SEC => "GiB/s",
            UnitCode::TEBIBYTES_PER_SEC => "TiB/s",
            UnitCode::CANDELAS => "cd",
            UnitCode::JOULES => "J",
            UnitCode::RADIANS => "rad",
            UnitCode::NEWTONS => "N",
            UnitCode::OHMS => "Ω",
            UnitCode::METRES_PER_SEC => "m/s",
            UnitCode::RADIANS_PER_SEC => "rad/s",
            UnitCode::METRES_PER_SEC_SQUARED => "m/s²",
            UnitCode::RADIANS_PER_SEC_SQUARED => "rad/s²",
            UnitCode::TESLAS => "T",
            UnitCode::WEBERS => "Wb",
            UnitCode::AMPS_PER_METRE => "A/m",
            UnitCode::METRES_SQUARED => "m²",
            UnitCode::METRES_CUBED => "m³",
            UnitCode::LITRES => "L",
            UnitCode::DB => "dB",
            UnitCode::DB_PEAK => "dB peak",
            UnitCode::DB_RMS => "dB RMS",
            UnitCode::DBFS => "dBFS",
            UnitCode::DBFS_PEAK => "dBFS peak",
            UnitCode::DBFS_RMS => "dBFS RMS",
            UnitCode::DBTP => "dBTP",
            UnitCode::DB_SPL_A => "dB(A) SPL",
            UnitCode::DB_Z => "dB(Z)",
            UnitCode::DB_SPL_C => "dB(C) SPL",
            UnitCode::DB_SPL => "dB SPL",
            UnitCode::LU => "LU",
            UnitCode::LUFS => "LUFS",
            UnitCode::DB_A => "dB(A)",
            _ => "",
        }
    }
}

/// A value's unit (7.3.4): its base quantity times a power of ten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Unit {
    pub multiplier: i8,
    pub code: UnitCode,
}

impl Unit {
    fn decode(bytes: &[u8]) -> Self {
        Self {
            multiplier: bytes[0] as i8,
            code: UnitCode(bytes[1]),
        }
    }

    /// What one of a raw value is worth in the base quantity: ten to the
    /// multiplier.
    pub fn scale(self) -> f64 {
        let mut scale = 1.0;
        for _ in 0..self.multiplier.unsigned_abs() {
            scale *= 10.0;
        }
        if self.multiplier < 0 {
            1.0 / scale
        } else {
            scale
        }
    }

    /// `number` in the base quantity, written with its suffix, such as
    /// "-6.0 dB" for -60 tenths of a decibel.
    pub fn show(self, number: Number) -> Shown {
        Shown { number, unit: self }
    }

    /// The raw value nearest `shown` in the base quantity, as `scalar`
    /// holds it.
    pub fn raw(self, shown: f64, scalar: Scalar) -> Number {
        number_for(shown / self.scale(), scalar)
    }
}

/// A value written in its unit's base quantity with its suffix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shown {
    number: Number,
    unit: Unit,
}

impl core::fmt::Display for Shown {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let value = self.number.to_f64() * self.unit.scale();
        // As many decimals as the multiplier gives, a few for a float.
        let decimals = match self.number {
            Number::Float(_) => 2,
            _ => usize::from((-self.unit.multiplier).clamp(0, 6) as u8),
        };
        write!(formatter, "{value:.decimals$}")?;
        match self.unit.code.suffix() {
            "" => Ok(()),
            "%" => formatter.write_str("%"),
            suffix => write!(formatter, " {suffix}"),
        }
    }
}

/// `raw` as `scalar` holds it: whole scalars take the nearest whole number.
fn number_for(raw: f64, scalar: Scalar) -> Number {
    // Half away from zero, as `as` cuts towards it.
    let whole = if raw < 0.0 { raw - 0.5 } else { raw + 0.5 };
    match scalar {
        Scalar::F32 | Scalar::F64 => Number::Float(raw),
        Scalar::U8 | Scalar::U16 | Scalar::U32 | Scalar::U64 => Number::Uint(whole.max(0.0) as u64),
        _ => Number::Int(whole as i64),
    }
}

/// The setting nearest `value` from `minimum` to `maximum` in `step`s.
fn nearest(
    value: Number,
    minimum: Number,
    maximum: Number,
    step: Number,
    scalar: Scalar,
) -> Number {
    let (low, high) = (minimum.to_f64(), maximum.to_f64());
    let (low, high) = if low <= high {
        (low, high)
    } else {
        (high, low)
    };
    let mut value = value.to_f64().clamp(low, high);
    let step = step.to_f64();
    if step > 0.0 {
        let steps = (value - low) / step;
        let steps = if steps < 0.0 {
            steps - 0.5
        } else {
            steps + 0.5
        } as i64;
        value = (low + steps as f64 * step).clamp(low, high);
    }
    number_for(value, scalar)
}

/// One value of a linear control (Table 7-122).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Linear {
    pub minimum: Number,
    pub maximum: Number,
    pub step: Number,
    pub default: Number,
    pub current: Number,
    pub unit: Unit,
    /// The value's name.
    pub string: LocalizedStringRef,
}

impl Linear {
    /// The setting nearest `value` the control can take.
    pub fn nearest(&self, value: Number, scalar: Scalar) -> Number {
        nearest(value, self.minimum, self.maximum, self.step, scalar)
    }
}

/// A selector control's value and options (Table 7-123); a string
/// selector's are localized string references.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Selector<'a> {
    pub current: Number,
    pub default: Number,
    pub unit: Unit,
    /// The options are localized string references.
    pub strings: bool,
    scalar: Scalar,
    options: &'a [u8],
}

impl<'a> Selector<'a> {
    pub fn options(&self) -> impl Iterator<Item = Number> + use<'a> {
        let scalar = self.scalar;
        self.options
            .chunks_exact(scalar.size())
            .map(move |option| scalar.read(option))
    }
}

/// An array control's shared range and its values (Table 7-124).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Array<'a> {
    pub minimum: Number,
    pub maximum: Number,
    pub step: Number,
    pub default: Number,
    pub unit: Unit,
    /// The first of a block of names, one for each value.
    pub string: LocalizedStringRef,
    scalar: Scalar,
    values: &'a [u8],
}

impl<'a> Array<'a> {
    /// The setting nearest `value` any of its values can take.
    pub fn nearest(&self, value: Number) -> Number {
        nearest(value, self.minimum, self.maximum, self.step, self.scalar)
    }

    pub fn current(&self) -> impl Iterator<Item = Number> + use<'a> {
        let scalar = self.scalar;
        self.values
            .chunks_exact(scalar.size())
            .map(move |value| scalar.read(value))
    }
}

/// A CONTROL descriptor (Table 7-38).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlDescriptor<'a> {
    pub index: u16,
    pub object_name: &'a str,
    pub localized_description: LocalizedStringRef,
    pub control_type: ControlType,
    pub value_type: ValueType,
    /// A controller may not change it.
    pub read_only: bool,
    /// The values it states are not known yet.
    pub unknown: bool,
    /// How long until it goes back to its default after being set, in
    /// microseconds; zero when it stays.
    pub reset_time: u32,
    /// What the control acts on: INVALID for nothing, as for IDENTIFY.
    pub signal_type: DescriptorType,
    pub signal_index: u16,
    pub signal_output: u16,
    /// number_of_values.
    count: u16,
    details: &'a [u8],
}

/// The flags in a control_value_type's top bits.
const READ_ONLY: u16 = 0x8000;
const UNKNOWN: u16 = 0x4000;
/// Where control_value_type is, and the fixed fields' end.
const VALUE_TYPE_AT: usize = 80;
const FIXED: usize = 104;

impl<'a> ControlDescriptor<'a> {
    pub fn decode(bytes: &'a [u8]) -> Result<Self, DecodeError> {
        let (raw_type, count, details) = layout(bytes)?;
        Ok(Self {
            index: read_u16(bytes, 2),
            object_name: aem_string(&bytes[4..68]),
            localized_description: LocalizedStringRef(read_u16(bytes, 68)),
            control_type: ControlType(u64::from_be_bytes(read_array(bytes, 82))),
            value_type: ValueType(raw_type & !(READ_ONLY | UNKNOWN)),
            read_only: raw_type & READ_ONLY != 0,
            unknown: raw_type & UNKNOWN != 0,
            reset_time: read_u32(bytes, 90),
            signal_type: DescriptorType(read_u16(bytes, 98)),
            signal_index: read_u16(bytes, 100),
            signal_output: read_u16(bytes, 102),
            count,
            details: &bytes[details],
        })
    }

    /// How many values it has: for a selector, how many options.
    pub fn count(&self) -> u16 {
        self.count
    }

    /// Each value of a linear control; none for another shape.
    pub fn linear(&self) -> impl Iterator<Item = Linear> + use<'a> {
        let scalar = match self.value_type.shape() {
            Shape::Linear => self.value_type.scalar(),
            _ => None,
        };
        let size = scalar.map_or(0, Scalar::size);
        let count = if scalar.is_some() {
            usize::from(self.count)
        } else {
            0
        };
        self.details
            .chunks_exact(5 * size + 4)
            .take(count)
            .filter_map(move |entry| {
                let scalar = scalar?;
                Some(Linear {
                    minimum: scalar.read(entry),
                    maximum: scalar.read(&entry[size..]),
                    step: scalar.read(&entry[2 * size..]),
                    default: scalar.read(&entry[3 * size..]),
                    current: scalar.read(&entry[4 * size..]),
                    unit: Unit::decode(&entry[5 * size..]),
                    string: LocalizedStringRef(read_u16(entry, 5 * size + 2)),
                })
            })
    }

    pub fn selector(&self) -> Option<Selector<'a>> {
        if self.value_type.shape() != Shape::Selector {
            return None;
        }
        let scalar = self.value_type.scalar()?;
        let size = scalar.size();
        let options_end = (usize::from(self.count) + 2) * size;
        Some(Selector {
            current: scalar.read(self.details),
            default: scalar.read(&self.details[size..]),
            unit: Unit::decode(&self.details[options_end..]),
            strings: self.value_type == ValueType::SELECTOR_STRING,
            scalar,
            options: &self.details[2 * size..options_end],
        })
    }

    pub fn array(&self) -> Option<Array<'a>> {
        if self.value_type.shape() != Shape::Array {
            return None;
        }
        let scalar = self.value_type.scalar()?;
        let size = scalar.size();
        let values = 4 * size + 4;
        Some(Array {
            minimum: scalar.read(self.details),
            maximum: scalar.read(&self.details[size..]),
            step: scalar.read(&self.details[2 * size..]),
            default: scalar.read(&self.details[3 * size..]),
            unit: Unit::decode(&self.details[4 * size..]),
            string: LocalizedStringRef(read_u16(self.details, 4 * size + 2)),
            scalar,
            values: &self.details[values..values + usize::from(self.count) * size],
        })
    }

    /// A text control's text.
    pub fn text(&self) -> Option<&'a str> {
        (self.value_type.shape() == Shape::Utf8).then(|| aem_string(self.details))
    }

    /// The current values, numbered, as SET_CONTROL takes them: each linear
    /// value's, the selector's one or the array's.
    pub fn current(&self) -> impl Iterator<Item = Number> + use<'a> {
        let selector = self.selector().map(|selector| selector.current);
        let array = self.array();
        self.linear()
            .map(|value| value.current)
            .chain(selector)
            .chain(array.into_iter().flat_map(|array| array.current()))
    }
}

/// The value type, count and value details of a CONTROL descriptor, checked
/// to hold what the type and count call for.
fn layout(bytes: &[u8]) -> Result<(u16, u16, core::ops::Range<usize>), DecodeError> {
    need(bytes, FIXED)?;
    let raw_type = read_u16(bytes, VALUE_TYPE_AT);
    let count = read_u16(bytes, 96);
    let start = usize::from(read_u16(bytes, 94));
    let value_type = ValueType(raw_type & !(READ_ONLY | UNKNOWN));
    let size = value_type.scalar().map_or(0, Scalar::size);
    let values = usize::from(count);
    let length = match value_type.shape() {
        Shape::Linear => values * (5 * size + 4),
        Shape::Selector => (values + 2) * size + 2,
        Shape::Array => (values + 4) * size + 4,
        Shape::Utf8 => 0,
        Shape::Other => match value_type {
            ValueType::SAMPLE_RATE => 4,
            ValueType::SMPTE_TIME | ValueType::GPTP_TIME => 10,
            ValueType::BODE_PLOT => values * 12 + 48,
            _ => 0,
        },
    };
    need(bytes, start + length)?;
    Ok((raw_type, count, start..bytes.len()))
}

fn need(bytes: &[u8], length: usize) -> Result<(), DecodeError> {
    if bytes.len() < length {
        return Err(DecodeError::Truncated {
            needed: length,
            available: bytes.len(),
        });
    }
    Ok(())
}

/// Writes the current `values` SET_CONTROL or GET_CONTROL carries into the
/// CONTROL descriptor `descriptor`, marking them known; whether it took
/// them, which it does not when they are too short or its values are a
/// Bode plot.
pub fn write_current(descriptor: &mut [u8], values: &[u8]) -> bool {
    let Ok((raw_type, count, details)) = layout(descriptor) else {
        return false;
    };
    let value_type = ValueType(raw_type & !(READ_ONLY | UNKNOWN));
    let size = value_type.scalar().map_or(0, Scalar::size);
    let count = usize::from(count);
    let details = &mut descriptor[details];
    match value_type.shape() {
        Shape::Linear => {
            if values.len() < count * size {
                return false;
            }
            let entry = 5 * size + 4;
            for (place, value) in values.chunks_exact(size).take(count).enumerate() {
                let at = place * entry + 4 * size;
                details[at..at + size].copy_from_slice(value);
            }
        }
        Shape::Selector => {
            let Some(value) = values.get(..size) else {
                return false;
            };
            details[..size].copy_from_slice(value);
        }
        Shape::Array => {
            let Some(current) = values.get(..count * size) else {
                return false;
            };
            let at = 4 * size + 4;
            details[at..at + current.len()].copy_from_slice(current);
        }
        Shape::Utf8 | Shape::Other if value_type != ValueType::BODE_PLOT => {
            // The text or value fills the details, the rest cleared.
            let length = values.len().min(details.len());
            details[..length].copy_from_slice(&values[..length]);
            details[length..].fill(0);
        }
        _ => return false,
    }
    let known = raw_type & !UNKNOWN;
    descriptor[VALUE_TYPE_AT..VALUE_TYPE_AT + 2].copy_from_slice(&known.to_be_bytes());
    true
}

/// The values `bytes` hold as `scalar`s, as SET_CONTROL carries them.
pub fn decode_values(scalar: Scalar, bytes: &[u8]) -> impl Iterator<Item = Number> + '_ {
    bytes
        .chunks_exact(scalar.size())
        .map(move |value| scalar.read(value))
}

/// Encodes `values` as `scalar`s into `out`, returning the length, or
/// `None` when `out` is too short.
pub fn encode_values(
    scalar: Scalar,
    values: impl IntoIterator<Item = Number>,
    out: &mut [u8],
) -> Option<usize> {
    let size = scalar.size();
    let mut length = 0;
    for value in values {
        scalar.write(value, out.get_mut(length..length + size)?);
        length += size;
    }
    Some(length)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A CONTROL descriptor as the wired endpoint answers it for its
    /// speaker volume: a linear 16-bit gain in tenths of a decibel.
    fn volume() -> [u8; 118] {
        let mut bytes = [0; 118];
        bytes[0..2].copy_from_slice(&DescriptorType::CONTROL.0.to_be_bytes());
        bytes[2..4].copy_from_slice(&1u16.to_be_bytes());
        bytes[4..10].copy_from_slice(b"Volume");
        bytes[68..70].copy_from_slice(&14u16.to_be_bytes());
        bytes[80..82].copy_from_slice(&ValueType::LINEAR_INT16.0.to_be_bytes());
        bytes[82..90].copy_from_slice(&ControlType::GAIN.0.to_be_bytes());
        bytes[94..96].copy_from_slice(&104u16.to_be_bytes());
        bytes[96..98].copy_from_slice(&1u16.to_be_bytes());
        bytes[98..100].copy_from_slice(&DescriptorType::INVALID.0.to_be_bytes());
        let details = [
            0xfc, 0x18, // minimum, -100.0 dB
            0x00, 0x3c, // maximum, 6.0 dB
            0x00, 0x05, // step, 0.5 dB
            0x00, 0x00, // default
            0xff, 0xc4, // current, -6.0 dB
            0xff, 0xb0, // unit: decibels times ten to the minus one
            0x00, 0x13, // string
        ];
        bytes[104..].copy_from_slice(&details);
        bytes
    }

    #[test]
    fn linear_controls_decode() {
        let bytes = volume();
        let control = ControlDescriptor::decode(&bytes).unwrap();
        assert_eq!(control.index, 1);
        assert_eq!(control.object_name, "Volume");
        assert_eq!(control.control_type, ControlType::GAIN);
        assert_eq!(control.value_type, ValueType::LINEAR_INT16);
        assert!(!control.read_only && !control.unknown);
        let values: Vec<Linear> = control.linear().collect();
        assert_eq!(
            values,
            [Linear {
                minimum: Number::Int(-1000),
                maximum: Number::Int(60),
                step: Number::Int(5),
                default: Number::Int(0),
                current: Number::Int(-60),
                unit: Unit {
                    multiplier: -1,
                    code: UnitCode::DB
                },
                string: LocalizedStringRef(19),
            }]
        );
        assert_eq!(control.current().collect::<Vec<_>>(), [Number::Int(-60)]);
        assert!(control.selector().is_none() && control.array().is_none());
        assert_eq!(UnitCode::DB.suffix(), "dB");
        // Value details cut short are refused.
        assert!(ControlDescriptor::decode(&bytes[..115]).is_err());
    }

    #[test]
    fn new_current_values_are_written_in_place() {
        let mut bytes = volume();
        bytes[80] |= 0x40;
        let mut values = [0; 2];
        let length = encode_values(Scalar::I16, [Number::Int(-120)], &mut values).unwrap();
        assert_eq!(
            decode_values(Scalar::I16, &values).collect::<Vec<_>>(),
            [Number::Int(-120)]
        );
        assert!(write_current(&mut bytes, &values[..length]));
        let control = ControlDescriptor::decode(&bytes).unwrap();
        assert_eq!(control.current().collect::<Vec<_>>(), [Number::Int(-120)]);
        assert!(!control.unknown, "a value heard is known");
        assert!(!write_current(&mut bytes, &[0]), "too short");
    }

    #[test]
    fn selectors_arrays_and_text_decode() {
        let mut bytes = [0; 120];
        bytes[80..82].copy_from_slice(&(ValueType::SELECTOR_UINT8.0 | READ_ONLY).to_be_bytes());
        bytes[94..96].copy_from_slice(&104u16.to_be_bytes());
        bytes[96..98].copy_from_slice(&3u16.to_be_bytes());
        // Current 50, default 60, options 50, 60, 70, unit hertz.
        bytes[104..111].copy_from_slice(&[50, 60, 50, 60, 70, 0x00, 0x10]);
        let control = ControlDescriptor::decode(&bytes).unwrap();
        assert!(control.read_only);
        let selector = control.selector().unwrap();
        assert_eq!(selector.current, Number::Uint(50));
        assert_eq!(selector.unit.code, UnitCode::HERTZ);
        assert_eq!(
            selector.options().collect::<Vec<_>>(),
            [Number::Uint(50), Number::Uint(60), Number::Uint(70)]
        );
        assert!(write_current(&mut bytes, &[70]));
        let control = ControlDescriptor::decode(&bytes).unwrap();
        assert_eq!(control.current().collect::<Vec<_>>(), [Number::Uint(70)]);

        let mut bytes = [0; 120];
        bytes[80..82].copy_from_slice(&ValueType::ARRAY_INT8.0.to_be_bytes());
        bytes[94..96].copy_from_slice(&104u16.to_be_bytes());
        bytes[96..98].copy_from_slice(&2u16.to_be_bytes());
        // Range -60 to 0, step 1, default -60, unit dBFS, string, -20, -6.
        bytes[104..114].copy_from_slice(&[0xc4, 0, 1, 0xc4, 0x00, 0xb3, 0, 0, 0xec, 0xfa]);
        let control = ControlDescriptor::decode(&bytes).unwrap();
        let array = control.array().unwrap();
        assert_eq!(array.unit.code, UnitCode::DBFS);
        assert_eq!(
            array.current().collect::<Vec<_>>(),
            [Number::Int(-20), Number::Int(-6)]
        );

        let mut bytes = [0; 130];
        bytes[80..82].copy_from_slice(&ValueType::UTF8.0.to_be_bytes());
        bytes[94..96].copy_from_slice(&104u16.to_be_bytes());
        bytes[96..98].copy_from_slice(&1u16.to_be_bytes());
        bytes[104..124].copy_from_slice(b"https://example.com\0");
        let control = ControlDescriptor::decode(&bytes).unwrap();
        assert_eq!(control.text(), Some("https://example.com"));
        assert!(write_current(&mut bytes, b"short\0"));
        let control = ControlDescriptor::decode(&bytes).unwrap();
        assert_eq!(control.text(), Some("short"));
    }

    #[test]
    fn values_show_in_their_units_and_snap_to_their_steps() {
        let tenths_of_db = Unit {
            multiplier: -1,
            code: UnitCode::DB,
        };
        assert_eq!(tenths_of_db.show(Number::Int(-60)).to_string(), "-6.0 dB");
        let percent = Unit {
            multiplier: 0,
            code: UnitCode::PERCENT,
        };
        assert_eq!(percent.show(Number::Uint(50)).to_string(), "50%");
        let kilohertz = Unit {
            multiplier: 3,
            code: UnitCode::HERTZ,
        };
        assert_eq!(kilohertz.show(Number::Uint(48)).to_string(), "48000 Hz");
        assert_eq!(tenths_of_db.raw(-12.04, Scalar::I16), Number::Int(-120));
        assert_eq!(tenths_of_db.raw(-0.06, Scalar::U8), Number::Uint(0));

        let bytes = volume();
        let control = ControlDescriptor::decode(&bytes).unwrap();
        let volume = control.linear().next().unwrap();
        // Steps of 0.5 dB, from -100 dB to 6 dB.
        assert_eq!(
            volume.nearest(Number::Int(-123), Scalar::I16),
            Number::Int(-125)
        );
        assert_eq!(
            volume.nearest(Number::Int(500), Scalar::I16),
            Number::Int(60)
        );
        assert_eq!(
            volume.nearest(Number::Int(-2000), Scalar::I16),
            Number::Int(-1000)
        );
    }

    #[test]
    fn values_saturate_at_their_scalars_range() {
        let mut out = [0; 2];
        Scalar::I8.write(Number::Int(-500), &mut out);
        assert_eq!(out[0] as i8, i8::MIN);
        Scalar::U16.write(Number::Float(70_000.0), &mut out);
        assert_eq!(u16::from_be_bytes(out), u16::MAX);
        assert_eq!(ValueType::ARRAY_FLOAT.scalar(), Some(Scalar::F32));
        assert_eq!(ValueType::SELECTOR_STRING.scalar(), Some(Scalar::U16));
        assert_eq!(ValueType::UTF8.scalar(), None);
    }
}
