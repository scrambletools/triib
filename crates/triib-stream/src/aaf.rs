//! AAF PDUs (IEEE 1722-2016, 7.2): the header and interleaved PCM
//! samples, big endian.

use atdecc::stream_format::{FormatKind, StreamFormat};

/// AAF's AVTP subtype.
pub const SUBTYPE: u8 = 0x02;
/// Octets of the AAF header.
pub const HEADER_LEN: usize = 24;

/// AAF sample formats (Table 9).
pub mod sample_format {
    pub const FLOAT_32: u8 = 1;
    pub const INT_32: u8 = 2;
    pub const INT_24: u8 = 3;
    pub const INT_16: u8 = 4;
}

/// What an AAF stream carries, from its stream format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub nsr: u8,
    pub sample_rate: u32,
    pub sample_format: u8,
    pub bit_depth: u8,
    pub channels: u16,
    pub samples_per_frame: u16,
}

impl Layout {
    /// The layout of an AAF stream format, when it is one this crate can
    /// send and receive: 16, 24 or 32-bit integers or 32-bit floats.
    pub fn of(format: StreamFormat) -> Option<Self> {
        let FormatKind::Aaf(aaf) = format.kind() else {
            return None;
        };
        let sample_rate = atdecc::stream_format::nsr_hertz(aaf.nsr)?;
        if aaf.channels == 0 || aaf.samples_per_frame == 0 {
            return None;
        }
        matches!(
            aaf.sample_format,
            sample_format::FLOAT_32
                | sample_format::INT_32
                | sample_format::INT_24
                | sample_format::INT_16
        )
        .then_some(Layout {
            nsr: aaf.nsr,
            sample_rate,
            sample_format: aaf.sample_format,
            bit_depth: aaf.bit_depth,
            channels: aaf.channels,
            samples_per_frame: aaf.samples_per_frame,
        })
    }

    /// Octets of one sample.
    pub fn sample_octets(&self) -> usize {
        match self.sample_format {
            sample_format::INT_24 => 3,
            sample_format::INT_16 => 2,
            _ => 4,
        }
    }

    /// Octets of samples in each frame.
    pub fn data_length(&self) -> usize {
        usize::from(self.channels) * usize::from(self.samples_per_frame) * self.sample_octets()
    }

    /// The AVTPDU's octets: header and samples, as MSRP's MaxFrameSize
    /// counts them.
    pub fn pdu_length(&self) -> usize {
        HEADER_LEN + self.data_length()
    }

    /// How often a frame goes out, in nanoseconds.
    pub fn interval_nanos(&self) -> u64 {
        1_000_000_000 * u64::from(self.samples_per_frame) / u64::from(self.sample_rate)
    }
}

/// An AAF header's fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub sequence: u8,
    pub stream_id: u64,
    /// The presentation time: the low 32 bits of gPTP nanoseconds, when
    /// valid.
    pub timestamp: Option<u32>,
    pub media_reset: bool,
    pub nsr: u8,
    pub sample_format: u8,
    pub bit_depth: u8,
    pub channels: u16,
    pub data_length: u16,
}

impl Header {
    pub fn encode(&self, out: &mut [u8; HEADER_LEN]) {
        out[0] = SUBTYPE;
        // sv, version 0, mr, gv 0, tv.
        out[1] =
            0x80 | if self.media_reset { 0x08 } else { 0 } | u8::from(self.timestamp.is_some());
        out[2] = self.sequence;
        out[3] = 0;
        out[4..12].copy_from_slice(&self.stream_id.to_be_bytes());
        out[12..16].copy_from_slice(&self.timestamp.unwrap_or(0).to_be_bytes());
        out[16] = self.sample_format;
        out[17] = (self.nsr << 4) | ((self.channels >> 8) as u8 & 0x03);
        out[18] = self.channels as u8;
        out[19] = self.bit_depth;
        out[20..22].copy_from_slice(&self.data_length.to_be_bytes());
        out[22] = 0; // sp 0: normal packing; evt 0
        out[23] = 0;
    }

    /// Decodes an AAF PDU, returning its header and samples.
    pub fn decode(pdu: &[u8]) -> Option<(Header, &[u8])> {
        let header: &[u8; HEADER_LEN] = pdu.get(..HEADER_LEN)?.try_into().ok()?;
        if header[0] != SUBTYPE || header[1] & 0x80 == 0 {
            return None;
        }
        let data_length = u16::from_be_bytes([header[20], header[21]]);
        let samples = pdu.get(HEADER_LEN..HEADER_LEN + usize::from(data_length))?;
        let mut stream_id = [0; 8];
        stream_id.copy_from_slice(&header[4..12]);
        Some((
            Header {
                sequence: header[2],
                stream_id: u64::from_be_bytes(stream_id),
                timestamp: (header[1] & 0x01 != 0)
                    .then(|| u32::from_be_bytes([header[12], header[13], header[14], header[15]])),
                media_reset: header[1] & 0x08 != 0,
                nsr: header[17] >> 4,
                sample_format: header[16],
                bit_depth: header[19],
                channels: (u16::from(header[17] & 0x03) << 8) | u16::from(header[18]),
                data_length,
            },
            samples,
        ))
    }

    /// Whether the frame carries what `layout` describes.
    pub fn matches(&self, layout: &Layout) -> bool {
        self.nsr == layout.nsr
            && self.sample_format == layout.sample_format
            && self.channels == layout.channels
    }
}

/// Writes `samples`, interleaved and from -1 to 1, as `layout`'s sample
/// format into `out`.
pub fn write_samples(samples: &[f32], layout: &Layout, out: &mut [u8]) {
    let octets = layout.sample_octets();
    for (sample, slot) in samples.iter().zip(out.chunks_exact_mut(octets)) {
        let clamped = sample.clamp(-1.0, 1.0);
        match layout.sample_format {
            sample_format::FLOAT_32 => slot.copy_from_slice(&clamped.to_be_bytes()),
            sample_format::INT_16 => {
                let value = (clamped * 32767.0) as i16;
                slot.copy_from_slice(&value.to_be_bytes());
            }
            sample_format::INT_24 => {
                let value = (f64::from(clamped) * 8_388_607.0) as i32;
                slot.copy_from_slice(&value.to_be_bytes()[1..]);
            }
            _ => {
                let value = (f64::from(clamped) * 2_147_483_647.0) as i32;
                slot.copy_from_slice(&value.to_be_bytes());
            }
        }
    }
}

/// Reads `layout`'s samples from `data` into `out`, from -1 to 1.
pub fn read_samples(data: &[u8], layout: &Layout, out: &mut Vec<f32>) {
    let octets = layout.sample_octets();
    out.extend(
        data.chunks_exact(octets)
            .map(|slot| match layout.sample_format {
                sample_format::FLOAT_32 => f32::from_be_bytes([slot[0], slot[1], slot[2], slot[3]]),
                sample_format::INT_16 => {
                    f32::from(i16::from_be_bytes([slot[0], slot[1]])) / 32768.0
                }
                sample_format::INT_24 => {
                    let value = i32::from_be_bytes([slot[0], slot[1], slot[2], 0]) >> 8;
                    value as f32 / 8_388_608.0
                }
                _ => {
                    let value = i32::from_be_bytes([slot[0], slot[1], slot[2], slot[3]]);
                    (f64::from(value) / 2_147_483_648.0) as f32
                }
            }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const AAF_48K_8CH_32: StreamFormat = StreamFormat(0x0205_0220_0200_6000);

    #[test]
    fn the_bench_format_is_six_frames_of_eight_channels() {
        let layout = Layout::of(AAF_48K_8CH_32).unwrap();
        assert_eq!(layout.sample_rate, 48_000);
        assert_eq!((layout.channels, layout.samples_per_frame), (8, 6));
        assert_eq!(layout.data_length(), 192);
        assert_eq!(layout.pdu_length(), 216);
        assert_eq!(layout.interval_nanos(), 125_000);
    }

    #[test]
    fn headers_and_samples_round_trip() {
        let layout = Layout::of(AAF_48K_8CH_32).unwrap();
        let header = Header {
            sequence: 7,
            stream_id: 0x9c6b_0030_9a2b_0000,
            timestamp: Some(0x1234_5678),
            media_reset: false,
            nsr: layout.nsr,
            sample_format: layout.sample_format,
            bit_depth: 32,
            channels: 8,
            data_length: layout.data_length() as u16,
        };
        let mut pdu = vec![0; layout.pdu_length()];
        let mut first = [0; HEADER_LEN];
        header.encode(&mut first);
        pdu[..HEADER_LEN].copy_from_slice(&first);
        let samples: Vec<f32> = (0..48).map(|place| place as f32 / 64.0 - 0.3).collect();
        write_samples(&samples, &layout, &mut pdu[HEADER_LEN..]);
        let (decoded, data) = Header::decode(&pdu).unwrap();
        assert_eq!(decoded, header);
        assert!(decoded.matches(&layout));
        let mut back = Vec::new();
        read_samples(data, &layout, &mut back);
        for (sent, received) in samples.iter().zip(&back) {
            assert!((sent - received).abs() < 1e-6, "{sent} {received}");
        }
    }

    #[test]
    fn other_pdus_do_not_decode() {
        assert!(Header::decode(&[0xfa; 40]).is_none());
        assert!(Header::decode(&[SUBTYPE, 0x81]).is_none());
        assert!(
            Layout::of(StreamFormat(0x0410_6001_0000_bb80)).is_none(),
            "CRF"
        );
    }
}
