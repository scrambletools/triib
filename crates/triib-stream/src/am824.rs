//! IEC 61883-6 AM824 PDUs (IEEE 1722-2016, 6.4): the 61883 header, the
//! CIP header and data blocks of one labelled quadlet per channel, each
//! a 24-bit sample after the multi-bit linear audio label.

use atdecc::stream_format::{FormatKind, Packing, StreamFormat};

/// The IEC 61883/IIDC AVTP subtype.
pub const SUBTYPE: u8 = 0x00;
/// Octets of the AVTP header, then of the CIP header after it.
pub const HEADER_LEN: usize = 24;
pub const CIP_LEN: usize = 8;
/// The CIP FMT of IEC 61883-6 audio and music.
const FMT: u8 = 0x10;
/// Multi-bit linear audio, 24 bits.
const MBLA_24: u8 = 0x40;
/// SYT when it carries nothing: the AVTP timestamp says when instead.
const NO_SYT: u16 = 0xffff;

/// What an AM824 stream carries, from its stream format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub sfc: u8,
    pub sample_rate: u32,
    pub channels: u16,
    pub samples_per_frame: u16,
}

impl Layout {
    /// The layout of an AM824 stream format, when this crate can send and
    /// receive it: non-blocking, every quadlet multi-bit linear audio, at
    /// a rate class A's 8000 frames a second divides.
    pub fn of(format: StreamFormat) -> Option<Self> {
        let FormatKind::Iec61883_6(iec) = format.kind() else {
            return None;
        };
        let sample_rate = atdecc::stream_format::sfc_hertz(iec.sfc)?;
        let usable = iec.packing == Packing::Am824
            && iec.non_blocking
            && iec.dbs > 0
            && iec.labels == [0, iec.dbs, 0, 0]
            && sample_rate % 8000 == 0;
        usable.then_some(Layout {
            sfc: iec.sfc,
            sample_rate,
            channels: u16::from(iec.dbs),
            samples_per_frame: (sample_rate / 8000) as u16,
        })
    }

    /// Octets of data blocks in each frame.
    pub fn data_length(&self) -> usize {
        usize::from(self.channels) * usize::from(self.samples_per_frame) * 4
    }

    /// The AVTPDU's octets, as MSRP's MaxFrameSize counts them.
    pub fn pdu_length(&self) -> usize {
        HEADER_LEN + CIP_LEN + self.data_length()
    }

    pub fn interval_nanos(&self) -> u64 {
        125_000
    }

    /// Data blocks between the instants SYT, and so the AVTP timestamp,
    /// can name (IEC 61883-6, Table 4).
    pub fn syt_interval(&self) -> u8 {
        match self.sample_rate {
            ..=48_000 => 8,
            48_001..=96_000 => 16,
            _ => 32,
        }
    }

    /// Which of a frame's data blocks its timestamp is for, when the
    /// first is block `dbc`: the one at an SYT_INTERVAL boundary, if the
    /// frame has one.
    pub fn stamped_block(&self, dbc: u8) -> Option<u16> {
        let interval = u16::from(self.syt_interval());
        let block = (interval - u16::from(dbc) % interval) % interval;
        (block < self.samples_per_frame).then_some(block)
    }
}

/// An AM824 PDU's header fields, the CIP header's among them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub sequence: u8,
    pub stream_id: u64,
    /// The presentation time of the data block at the frame's SYT_INTERVAL
    /// boundary, the low 32 bits of gPTP nanoseconds; none without one.
    pub timestamp: Option<u32>,
    pub media_reset: bool,
    /// Quadlets in each data block.
    pub dbs: u8,
    pub sfc: u8,
    /// The first data block's count.
    pub dbc: u8,
    /// Octets of data blocks.
    pub data_length: u16,
}

impl Header {
    pub fn encode(&self, out: &mut [u8; HEADER_LEN + CIP_LEN]) {
        out[0] = SUBTYPE;
        // sv, version 0, mr, gv 0, tv.
        out[1] =
            0x80 | if self.media_reset { 0x08 } else { 0 } | u8::from(self.timestamp.is_some());
        out[2] = self.sequence;
        out[3] = 0;
        out[4..12].copy_from_slice(&self.stream_id.to_be_bytes());
        out[12..16].copy_from_slice(&self.timestamp.unwrap_or(0).to_be_bytes());
        // No gateway.
        out[16..20].fill(0);
        let length = CIP_LEN as u16 + self.data_length;
        out[20..22].copy_from_slice(&length.to_be_bytes());
        // Tag 1 (a CIP header follows), channel 31 (native AVTP), then
        // tcode 0xA and sy 0.
        out[22] = 0x40 | 31;
        out[23] = 0xa0;
        // SID 63, DBS, FN, QPC and SPH 0, DBC.
        out[24] = 0x3f;
        out[25] = self.dbs;
        out[26] = 0;
        out[27] = self.dbc;
        // EOH, FMT, FDF (AM824 and the rate) and SYT.
        out[28] = 0x80 | FMT;
        out[29] = self.sfc & 0x07;
        out[30..32].copy_from_slice(&NO_SYT.to_be_bytes());
    }

    /// Decodes an AM824 PDU, returning its header and data blocks.
    pub fn decode(pdu: &[u8]) -> Option<(Header, &[u8])> {
        let header: &[u8; HEADER_LEN + CIP_LEN] =
            pdu.get(..HEADER_LEN + CIP_LEN)?.try_into().ok()?;
        if header[0] != SUBTYPE || header[1] & 0x80 == 0 || header[22] & 0xc0 != 0x40 {
            return None;
        }
        if header[28] & 0x3f != FMT {
            return None;
        }
        let length = usize::from(u16::from_be_bytes([header[20], header[21]]));
        let data = pdu.get(HEADER_LEN + CIP_LEN..HEADER_LEN + length.max(CIP_LEN))?;
        let mut stream_id = [0; 8];
        stream_id.copy_from_slice(&header[4..12]);
        Some((
            Header {
                sequence: header[2],
                stream_id: u64::from_be_bytes(stream_id),
                timestamp: (header[1] & 0x01 != 0)
                    .then(|| u32::from_be_bytes([header[12], header[13], header[14], header[15]])),
                media_reset: header[1] & 0x08 != 0,
                dbs: header[25],
                sfc: header[29] & 0x07,
                dbc: header[27],
                data_length: data.len() as u16,
            },
            data,
        ))
    }

    /// Whether the frame carries what `layout` describes.
    pub fn matches(&self, layout: &Layout) -> bool {
        self.sfc == layout.sfc && u16::from(self.dbs) == layout.channels
    }
}

/// Writes `samples`, interleaved and from -1 to 1, as labelled 24-bit
/// quadlets into `out`.
pub fn write_samples(samples: &[f32], out: &mut [u8]) {
    for (sample, quadlet) in samples.iter().zip(out.as_chunks_mut::<4>().0) {
        let value = (f64::from(sample.clamp(-1.0, 1.0)) * 8_388_607.0) as i32;
        quadlet[0] = MBLA_24;
        quadlet[1..].copy_from_slice(&value.to_be_bytes()[1..]);
    }
}

/// Reads the samples of `data`'s quadlets into `out`, from -1 to 1;
/// quadlets of another kind than multi-bit linear audio read as silence.
pub fn read_samples(data: &[u8], out: &mut Vec<f32>) {
    out.extend(data.as_chunks::<4>().0.iter().map(|quadlet| {
        if quadlet[0] & 0xfc != MBLA_24 {
            return 0.0;
        }
        let value = i32::from_be_bytes([quadlet[1], quadlet[2], quadlet[3], 0]) >> 8;
        value as f32 / 8_388_608.0
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Mac mini's AM824 input: 48 kHz, 8 channels, non-blocking.
    const AM824_48K_8CH: StreamFormat = StreamFormat(0x00a0_0208_4000_0800);

    #[test]
    fn the_bench_format_is_six_blocks_of_eight_quadlets() {
        let layout = Layout::of(AM824_48K_8CH).unwrap();
        assert_eq!(layout.sample_rate, 48_000);
        assert_eq!((layout.channels, layout.samples_per_frame), (8, 6));
        assert_eq!(layout.data_length(), 192);
        assert_eq!(layout.pdu_length(), 224);
        assert_eq!(layout.syt_interval(), 8);
        // Blocks 0, 6, 12 and 18 start the frames of 24 blocks: three of
        // the four have an SYT_INTERVAL boundary.
        let stamped: Vec<_> = [0, 6, 12, 18].map(|dbc| layout.stamped_block(dbc)).to_vec();
        assert_eq!(stamped, [Some(0), Some(2), Some(4), None]);
        // Blocking is not taken; synchronous beside non-blocking is.
        assert!(Layout::of(StreamFormat(0x00a0_0208_8000_0800)).is_none());
        assert!(Layout::of(StreamFormat(0x00a0_0208_5000_0800)).is_some());
    }

    #[test]
    fn headers_and_samples_round_trip() {
        let layout = Layout::of(AM824_48K_8CH).unwrap();
        let header = Header {
            sequence: 9,
            stream_id: 0x9c6b_0030_9a2b_0000,
            timestamp: Some(0x1234_5678),
            media_reset: true,
            dbs: 8,
            sfc: layout.sfc,
            dbc: 0xfa,
            data_length: layout.data_length() as u16,
        };
        let mut pdu = vec![0; layout.pdu_length()];
        let mut first = [0; HEADER_LEN + CIP_LEN];
        header.encode(&mut first);
        assert_eq!(
            &first[22..32],
            &[0x5f, 0xa0, 0x3f, 8, 0, 0xfa, 0x90, 2, 0xff, 0xff]
        );
        pdu[..HEADER_LEN + CIP_LEN].copy_from_slice(&first);
        let samples: Vec<f32> = (0..48).map(|place| place as f32 / 64.0 - 0.3).collect();
        write_samples(&samples, &mut pdu[HEADER_LEN + CIP_LEN..]);
        assert_eq!(pdu[HEADER_LEN + CIP_LEN], 0x40);
        let (decoded, data) = Header::decode(&pdu).unwrap();
        assert_eq!(decoded, header);
        assert!(decoded.matches(&layout));
        let mut back = Vec::new();
        read_samples(data, &mut back);
        assert_eq!(back.len(), samples.len());
        for (sent, received) in samples.iter().zip(&back) {
            assert!((sent - received).abs() < 1e-6, "{sent} {received}");
        }
    }

    #[test]
    fn aaf_pdus_do_not_decode() {
        assert!(Header::decode(&[0x02; 64]).is_none());
        let mut no_cip = [0u8; 64];
        no_cip[1] = 0x80;
        assert!(Header::decode(&no_cip).is_none());
    }
}
