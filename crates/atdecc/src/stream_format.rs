//! Stream formats (IEEE 1722-2016, Annex I; IEEE 1722.1-2021, 7.3.3):
//! the 64-bit description of what a stream carries, decoded for the
//! formats audio devices use, and whether a talker's stream suits a
//! listener.

use core::fmt;

/// A stream format, kept as its 64 bits.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct StreamFormat(pub u64);

/// AVTP subtypes that appear in stream formats (IEEE 1722-2016, Table 6).
mod subtype {
    pub const IEC_61883_IIDC: u8 = 0x00;
    pub const AAF: u8 = 0x02;
    pub const CRF: u8 = 0x04;
}

/// What a stream format describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatKind {
    /// AVTP Audio Format (I.2.4).
    Aaf(Aaf),
    /// IEC 61883-6 audio and music data (I.2.2.3.3).
    Iec61883_6(Iec61883_6),
    /// Clock Reference Format (I.2.6).
    Crf(Crf),
    /// A vendor defined format (v set).
    Vendor,
    /// Another AVTP subtype, not decoded.
    Other { subtype: u8 },
}

/// An AAF stream format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aaf {
    /// The source or sink can use fewer channels than `channels`.
    pub up_to: bool,
    /// The nominal sample rate code (IEEE 1722-2016, Table 11).
    pub nsr: u8,
    /// The sample format code (IEEE 1722-2016, Table 9).
    pub sample_format: u8,
    pub bit_depth: u8,
    pub channels: u16,
    pub samples_per_frame: u16,
}

/// An IEC 61883-6 stream format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Iec61883_6 {
    /// The packetization: AM824, 32-bit fixed point or 32-bit float.
    pub packing: Packing,
    /// The sampling frequency code of the FDF field.
    pub sfc: u8,
    /// Data block size: quadlets, so channels, per data block.
    pub dbs: u8,
    pub blocking: bool,
    pub non_blocking: bool,
    pub up_to: bool,
    pub synchronous: bool,
    /// For AM824: IEC 60958, multi-bit linear audio, MIDI and SMPTE quadlet
    /// counts.
    pub labels: [u8; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Packing {
    Am824,
    Fixed32,
    Float32,
    Other(u8),
}

/// A CRF stream format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crf {
    /// The clock type (IEEE 1722-2016, Table 26): 1 for audio samples.
    pub crf_type: u8,
    /// Events between timestamps.
    pub timestamp_interval: u16,
    pub timestamps_per_pdu: u8,
    pub pull: u8,
    pub base_frequency: u32,
}

impl StreamFormat {
    fn octet(self, index: usize) -> u8 {
        self.0.to_be_bytes()[index]
    }

    pub fn kind(self) -> FormatKind {
        let first = self.octet(0);
        if first & 0x80 != 0 {
            return FormatKind::Vendor;
        }
        match first & 0x7f {
            subtype::AAF => {
                let low = self.0 as u32;
                FormatKind::Aaf(Aaf {
                    up_to: self.octet(1) & 0x80 != 0,
                    nsr: self.octet(1) & 0x0f,
                    sample_format: self.octet(2),
                    bit_depth: self.octet(3),
                    channels: (low >> 22) as u16,
                    samples_per_frame: ((low >> 12) & 0x3ff) as u16,
                })
            }
            subtype::IEC_61883_IIDC => {
                let sf = self.octet(1) & 0x80 != 0;
                let fmt = (self.octet(1) >> 1) & 0x3f;
                if !sf || fmt != 0x10 {
                    return FormatKind::Other {
                        subtype: subtype::IEC_61883_IIDC,
                    };
                }
                let fdf = self.octet(2);
                let flags = self.octet(4);
                FormatKind::Iec61883_6(Iec61883_6 {
                    packing: match fdf >> 3 {
                        0b00000 => Packing::Am824,
                        0b00110 => Packing::Fixed32,
                        0b00100 => Packing::Float32,
                        other => Packing::Other(other),
                    },
                    sfc: fdf & 0x07,
                    dbs: self.octet(3),
                    blocking: flags & 0x80 != 0,
                    non_blocking: flags & 0x40 != 0,
                    up_to: flags & 0x20 != 0,
                    synchronous: flags & 0x10 != 0,
                    labels: [
                        self.octet(5),
                        self.octet(6),
                        self.octet(7) >> 4,
                        self.octet(7) & 0x0f,
                    ],
                })
            }
            subtype::CRF => {
                let high = (self.0 >> 32) as u32;
                let low = self.0 as u32;
                FormatKind::Crf(Crf {
                    crf_type: ((high >> 20) & 0x0f) as u8,
                    timestamp_interval: ((high >> 8) & 0x0fff) as u16,
                    timestamps_per_pdu: high as u8,
                    pull: (low >> 29) as u8,
                    base_frequency: low & 0x1fff_ffff,
                })
            }
            other => FormatKind::Other { subtype: other },
        }
    }

    /// The nominal sample or clock rate in hertz, where the format gives
    /// one.
    pub fn sample_rate(self) -> Option<u32> {
        match self.kind() {
            FormatKind::Aaf(aaf) => nsr_hertz(aaf.nsr),
            FormatKind::Iec61883_6(iec) => sfc_hertz(iec.sfc),
            FormatKind::Crf(crf) => (crf.pull == 0).then_some(crf.base_frequency),
            _ => None,
        }
    }

    /// Audio channels, for audio formats.
    pub fn channels(self) -> Option<u16> {
        match self.kind() {
            FormatKind::Aaf(aaf) => Some(aaf.channels),
            FormatKind::Iec61883_6(iec) => Some(u16::from(iec.dbs)),
            _ => None,
        }
    }

    /// Whether the stream carries a media clock rather than audio.
    pub fn is_clock(self) -> bool {
        matches!(self.kind(), FormatKind::Crf(_))
    }

    /// An AAF format without its up-to flag and channel count, which may
    /// differ between a talker and a listener that takes fewer channels.
    fn aaf_without_channels(self) -> u64 {
        const UP_TO: u64 = 0x0080_0000_0000_0000;
        const CHANNELS: u64 = 0x3ff << 22;
        self.0 & !UP_TO & !CHANNELS
    }

    /// Whether a stream in `self`, a talker's current format, can be
    /// received by a stream input that supports `listener`: the same
    /// format, or one the listener takes with fewer channels.
    pub fn suits(self, listener: StreamFormat) -> bool {
        if self == listener {
            return true;
        }
        match (self.kind(), listener.kind()) {
            (FormatKind::Aaf(talker), FormatKind::Aaf(sink)) => {
                self.aaf_without_channels() == listener.aaf_without_channels()
                    && (talker.channels == sink.channels
                        || sink.up_to && talker.channels <= sink.channels)
            }
            (FormatKind::Iec61883_6(talker), FormatKind::Iec61883_6(sink)) => {
                talker.packing == sink.packing
                    && talker.sfc == sink.sfc
                    && (talker.dbs == sink.dbs || sink.up_to && talker.dbs <= sink.dbs)
                    && (sink.blocking && talker.blocking
                        || sink.non_blocking && talker.non_blocking)
                    && (talker.packing != Packing::Am824
                        || talker.labels == sink.labels
                        || sink.up_to)
            }
            _ => false,
        }
    }
}

/// How a talker's stream fits a listener's stream input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// The input's current format is the talker's.
    Matches,
    /// The input supports the talker's format but is set to another.
    InputMustChange,
    /// The input does not support the talker's format.
    Incompatible,
}

/// How a talker's current format fits a stream input with `current`
/// format and `supported` formats.
pub fn fit(
    talker: StreamFormat,
    current: StreamFormat,
    supported: impl IntoIterator<Item = StreamFormat>,
) -> Fit {
    if talker.suits(current) {
        Fit::Matches
    } else if supported.into_iter().any(|format| talker.suits(format)) {
        Fit::InputMustChange
    } else {
        Fit::Incompatible
    }
}

/// The rate an AAF nsr code stands for (IEEE 1722-2016, Table 11).
pub fn nsr_hertz(nsr: u8) -> Option<u32> {
    Some(match nsr {
        1 => 8_000,
        2 => 16_000,
        3 => 32_000,
        4 => 44_100,
        5 => 48_000,
        6 => 88_200,
        7 => 96_000,
        8 => 176_400,
        9 => 192_000,
        10 => 24_000,
        _ => return None,
    })
}

/// The rate an IEC 61883-6 SFC code stands for.
pub fn sfc_hertz(sfc: u8) -> Option<u32> {
    Some(match sfc {
        0 => 32_000,
        1 => 44_100,
        2 => 48_000,
        3 => 88_200,
        4 => 96_000,
        5 => 176_400,
        6 => 192_000,
        _ => return None,
    })
}

/// "48 kHz", "44.1 kHz".
struct Rate(u32);

impl fmt::Display for Rate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 < 1000 {
            write!(formatter, "{} Hz", self.0)
        } else if self.0.is_multiple_of(1000) {
            write!(formatter, "{} kHz", self.0 / 1000)
        } else {
            write!(formatter, "{}.{} kHz", self.0 / 1000, self.0 % 1000 / 100)
        }
    }
}

impl fmt::Display for StreamFormat {
    /// A short description, such as "AAF 48 kHz, 8 ch, 32-bit (24 used)".
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rate = self.sample_rate().map(Rate);
        match self.kind() {
            FormatKind::Aaf(aaf) => {
                formatter.write_str("AAF ")?;
                if let Some(rate) = rate {
                    write!(formatter, "{rate}, ")?;
                }
                write!(formatter, "{} ch", aaf.channels)?;
                if aaf.up_to {
                    formatter.write_str(" or fewer")?;
                }
                let size = match aaf.sample_format {
                    1 => Some((32, true)),
                    2 => Some((32, false)),
                    3 => Some((24, false)),
                    4 => Some((16, false)),
                    _ => None,
                };
                match size {
                    Some((bits, true)) => write!(formatter, ", {bits}-bit float"),
                    Some((bits, false))
                        if aaf.bit_depth != 0 && u32::from(aaf.bit_depth) < bits =>
                    {
                        write!(formatter, ", {bits}-bit ({} used)", aaf.bit_depth)
                    }
                    Some((bits, false)) => write!(formatter, ", {bits}-bit"),
                    None => write!(formatter, ", sample format {}", aaf.sample_format),
                }
            }
            FormatKind::Iec61883_6(iec) => {
                let packing = match iec.packing {
                    Packing::Am824 => "AM824",
                    Packing::Fixed32 => "IEC 61883-6 32-bit",
                    Packing::Float32 => "IEC 61883-6 float",
                    Packing::Other(_) => "IEC 61883-6",
                };
                write!(formatter, "{packing} ")?;
                if let Some(rate) = rate {
                    write!(formatter, "{rate}, ")?;
                }
                write!(formatter, "{} ch", iec.dbs)?;
                if iec.up_to {
                    formatter.write_str(" or fewer")?;
                }
                // Non-blocking and asynchronous are what AVB streams use;
                // say so only when a format differs.
                if iec.blocking {
                    formatter.write_str(", blocking")?;
                }
                if iec.synchronous {
                    formatter.write_str(", synchronous")?;
                }
                Ok(())
            }
            FormatKind::Crf(crf) => {
                formatter.write_str("CRF ")?;
                let (kind, event) = match crf.crf_type {
                    1 => ("audio", "sample"),
                    2 => ("video frame", "frame"),
                    3 => ("video line", "line"),
                    4 => ("machine cycle", "cycle"),
                    _ => ("clock", "event"),
                };
                write!(formatter, "{kind} {}", Rate(crf.base_frequency))?;
                // The pull multiplies the base frequency (IEEE 1722-2016,
                // Table 27).
                match crf.pull {
                    0 => {}
                    1 => formatter.write_str(" × 1/1.001")?,
                    2 => formatter.write_str(" × 1.001")?,
                    3 => formatter.write_str(" × 24/25")?,
                    4 => formatter.write_str(" × 25/24")?,
                    5 => formatter.write_str(" × 1/8")?,
                    pull => write!(formatter, ", pull {pull}")?,
                }
                match crf.timestamp_interval {
                    1 => write!(formatter, ", every {event}"),
                    interval => write!(formatter, ", every {interval} {event}s"),
                }
            }
            FormatKind::Vendor => write!(formatter, "vendor format 0x{:016x}", self.0),
            FormatKind::Other { subtype } => {
                write!(
                    formatter,
                    "subtype 0x{subtype:02x} format 0x{:016x}",
                    self.0
                )
            }
        }
    }
}

impl fmt::Debug for StreamFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "StreamFormat(0x{:016x})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_aaf_from_a_milan_endpoint() {
        let format = StreamFormat(0x0205_0218_0200_6000);
        let FormatKind::Aaf(aaf) = format.kind() else {
            panic!("{format:?}");
        };
        assert_eq!((aaf.nsr, aaf.sample_format, aaf.bit_depth), (5, 2, 24));
        assert_eq!((aaf.channels, aaf.samples_per_frame), (8, 6));
        assert_eq!(format.sample_rate(), Some(48_000));
        assert_eq!(format.to_string(), "AAF 48 kHz, 8 ch, 32-bit (24 used)");
        assert_eq!(
            StreamFormat(0x0209_0218_0201_8000).to_string(),
            "AAF 192 kHz, 8 ch, 32-bit (24 used)"
        );
    }

    #[test]
    fn decodes_am824_from_macos() {
        let format = StreamFormat(0x00a0_0608_4000_0800);
        let FormatKind::Iec61883_6(iec) = format.kind() else {
            panic!("{format:?}");
        };
        assert_eq!(iec.packing, Packing::Am824);
        assert_eq!((iec.sfc, iec.dbs), (6, 8));
        assert!(iec.non_blocking && !iec.blocking);
        assert_eq!(iec.labels, [0, 8, 0, 0]);
        assert_eq!(format.to_string(), "AM824 192 kHz, 8 ch");
        // The same with the synchronous flag set reads apart.
        assert_eq!(
            StreamFormat(0x00a0_0608_5000_0800).to_string(),
            "AM824 192 kHz, 8 ch, synchronous"
        );
    }

    #[test]
    fn decodes_crf() {
        let format = StreamFormat(0x0410_6001_0000_bb80);
        let FormatKind::Crf(crf) = format.kind() else {
            panic!("{format:?}");
        };
        assert_eq!(
            (crf.crf_type, crf.timestamp_interval, crf.timestamps_per_pdu),
            (1, 96, 1)
        );
        assert_eq!(crf.base_frequency, 48_000);
        assert!(format.is_clock());
        assert_eq!(format.to_string(), "CRF audio 48 kHz, every 96 samples");
        // Video frame clocks run below a kilohertz, some pulled.
        assert_eq!(
            StreamFormat(0x0420_0101_0000_0018).to_string(),
            "CRF video frame 24 Hz, every frame"
        );
        assert_eq!(
            StreamFormat(0x0420_0101_2000_0018).to_string(),
            "CRF video frame 24 Hz × 1/1.001, every frame"
        );
    }

    #[test]
    fn fits_by_format_and_channels() {
        let eight = StreamFormat(0x0205_0220_0200_6000);
        let two = StreamFormat(0x0205_0220_0080_6000);
        let eight_up_to = StreamFormat(eight.0 | 0x0080_0000_0000_0000);
        let other_rate = StreamFormat(0x0207_0220_0200_c000);
        let clock = StreamFormat(0x0410_6001_0000_bb80);
        assert_eq!(fit(eight, eight, []), Fit::Matches);
        assert_eq!(fit(two, eight_up_to, []), Fit::Matches);
        assert_eq!(fit(two, eight, [two]), Fit::InputMustChange);
        assert_eq!(fit(eight, two, []), Fit::Incompatible);
        assert_eq!(fit(other_rate, eight, [eight, two]), Fit::Incompatible);
        assert_eq!(fit(clock, eight, [eight]), Fit::Incompatible);
    }
}
