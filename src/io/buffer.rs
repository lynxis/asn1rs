use crate::io::uper::Error as UperError;
use crate::io::uper::Reader as UperReader;
use crate::io::uper::Writer as UperWriter;
use crate::io::uper::BYTE_LEN;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Default)]
pub struct BitBuffer {
    buffer: Vec<u8>,
    write_position: usize,
    read_position: usize,
}

impl BitBuffer {
    pub fn from_bytes(buffer: Vec<u8>) -> BitBuffer {
        let bits = buffer.len() * 8;
        Self::from_bits(buffer, bits)
    }

    pub fn from_bits(buffer: Vec<u8>, bit_length: usize) -> BitBuffer {
        assert!(bit_length <= buffer.len() * 8);
        BitBuffer {
            buffer,
            write_position: bit_length,
            read_position: 0,
        }
    }

    #[allow(unused)]
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.write_position = 0;
        self.read_position = 0;
    }

    #[allow(unused)]
    pub fn reset_read_position(&mut self) {
        self.read_position = 0;
    }

    #[allow(unused)]
    pub fn content(&self) -> &[u8] {
        &self.buffer[..]
    }

    #[allow(unused)]
    pub fn bit_len(&self) -> usize {
        self.write_position
    }

    #[allow(unused)]
    pub fn byte_len(&self) -> usize {
        self.buffer.len()
    }
}

impl Into<Vec<u8>> for BitBuffer {
    fn into(self) -> Vec<u8> {
        self.buffer
    }
}

impl From<Vec<u8>> for BitBuffer {
    fn from(buffer: Vec<u8>) -> Self {
        Self::from_bytes(buffer)
    }
}

impl UperReader for BitBuffer {
    fn read_bit(&mut self) -> Result<bool, UperError> {
        if self.read_position >= self.write_position {
            return Err(UperError::EndOfStream);
        }
        let byte_pos = self.read_position as usize / BYTE_LEN;
        let bit_pos = self.read_position % BYTE_LEN;
        let bit_pos = (BYTE_LEN - bit_pos - 1) as u8; // flip
        let mask = 0x01 << bit_pos;
        let bit = (self.buffer[byte_pos] & mask) == mask;
        self.read_position += 1;
        Ok(bit)
    }
}

impl UperWriter for BitBuffer {
    fn write_bit(&mut self, bit: bool) -> Result<(), UperError> {
        let byte_pos = self.write_position as usize / BYTE_LEN;
        let bit_pos = self.write_position % BYTE_LEN;
        if bit_pos == 0 {
            self.buffer.push(0x00);
        }
        if bit {
            let bit_pos = (BYTE_LEN - bit_pos - 1) as u8; // flip
            self.buffer[byte_pos] |= 0x01 << bit_pos;
        }
        self.write_position += 1;
        Ok(())
    }
}

#[allow(clippy::identity_op)] // for better readability across multiple tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn bit_buffer_write_bit_keeps_correct_order() -> Result<(), UperError> {
        let mut buffer = BitBuffer::default();

        buffer.write_bit(true)?;
        buffer.write_bit(false)?;
        buffer.write_bit(false)?;
        buffer.write_bit(true)?;

        buffer.write_bit(true)?;
        buffer.write_bit(true)?;
        buffer.write_bit(true)?;
        buffer.write_bit(false)?;

        assert_eq!(buffer.content(), &[0b1001_1110]);

        buffer.write_bit(true)?;
        buffer.write_bit(false)?;
        buffer.write_bit(true)?;
        buffer.write_bit(true)?;

        buffer.write_bit(true)?;
        buffer.write_bit(true)?;
        buffer.write_bit(true)?;
        buffer.write_bit(false)?;

        assert_eq!(buffer.content(), &[0b1001_1110, 0b1011_1110]);

        buffer.write_bit(true)?;
        buffer.write_bit(false)?;
        buffer.write_bit(true)?;
        buffer.write_bit(false)?;

        assert_eq!(buffer.content(), &[0b1001_1110, 0b1011_1110, 0b1010_0000]);

        let mut buffer = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
        assert!(buffer.read_bit()?);
        assert!(!buffer.read_bit()?);
        assert!(!buffer.read_bit()?);
        assert!(buffer.read_bit()?);

        assert!(buffer.read_bit()?);
        assert!(buffer.read_bit()?);
        assert!(buffer.read_bit()?);
        assert!(!buffer.read_bit()?);

        assert!(buffer.read_bit()?);
        assert!(!buffer.read_bit()?);
        assert!(buffer.read_bit()?);
        assert!(buffer.read_bit()?);

        assert!(buffer.read_bit()?);
        assert!(buffer.read_bit()?);
        assert!(buffer.read_bit()?);
        assert!(!buffer.read_bit()?);

        assert!(buffer.read_bit()?);
        assert!(!buffer.read_bit()?);
        assert!(buffer.read_bit()?);
        assert!(!buffer.read_bit()?);

        assert_eq!(buffer.read_bit(), Err(UperError::EndOfStream));

        Ok(())
    }

    #[test]
    fn bit_buffer_bit_string_till_end() -> Result<(), UperError> {
        let content = &[0xFF, 0x74, 0xA6, 0x0F];
        let mut buffer = BitBuffer::default();
        buffer.write_bit_string_till_end(content, 0)?;
        assert_eq!(buffer.content(), content);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            let mut content2 = vec![0_u8; content.len()];
            buffer2.read_bit_string_till_end(&mut content2[..], 0)?;
            assert_eq!(&content[..], &content2[..]);
        }

        let mut content2 = vec![0xFF_u8; content.len()];
        buffer.read_bit_string_till_end(&mut content2[..], 0)?;
        assert_eq!(&content[..], &content2[..]);

        Ok(())
    }

    #[test]
    fn bit_buffer_bit_string_till_end_with_offset() -> Result<(), UperError> {
        let content = &[0b1111_1111, 0b0111_0100, 0b1010_0110, 0b0000_1111];
        let mut buffer = BitBuffer::default();
        buffer.write_bit_string_till_end(content, 7)?;
        assert_eq!(
            buffer.content(),
            &[0b1011_1010, 0b0101_0011, 0b0000_0111, 0b1000_0000]
        );

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            let mut content2 = vec![0xFF_u8; content.len()];
            content2[0] = content[0] & 0b1111_1110; // since we are skipping the first 7 bits
            buffer2.read_bit_string_till_end(&mut content2[..], 7)?;
            assert_eq!(&content[..], &content2[..]);
        }

        let mut content2 = vec![0_u8; content.len()];
        content2[0] = content[0] & 0b1111_1110; // since we are skipping the first 7 bits
        buffer.read_bit_string_till_end(&mut content2[..], 7)?;
        assert_eq!(&content[..], &content2[..]);

        Ok(())
    }

    #[test]
    fn bit_buffer_bit_string() -> Result<(), UperError> {
        let content = &[0b1111_1111, 0b0111_0100, 0b1010_0110, 0b0000_1111];
        let mut buffer = BitBuffer::default();
        buffer.write_bit_string(content, 7, 12)?;
        assert_eq!(buffer.content(), &[0b1011_1010, 0b0101_0000]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            let mut content2 = vec![0_u8; content.len()];
            // since we are skipping the first 7 bits
            let content = &[
                content[0] & 0x01,
                content[1],
                content[2] & 0b1110_0000,
                0x00,
            ];
            buffer2.read_bit_string(&mut content2[..], 7, 12)?;
            assert_eq!(&content[..], &content2[..]);
        }

        let mut content2 = vec![0x00_u8; content.len()];
        // since we are skipping the first 7 bits
        let content = &[
            content[0] & 0x01,
            content[1],
            content[2] & 0b1110_0000,
            0x00,
        ];
        buffer.read_bit_string(&mut content2[..], 7, 12)?;
        assert_eq!(&content[..], &content2[..]);

        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_0() -> Result<(), UperError> {
        const DET: usize = 0;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(DET)?;
        assert_eq!(buffer.content(), &[0x00 | DET as u8]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant()?);
        }

        assert_eq!(DET, buffer.read_length_determinant()?);

        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_1() -> Result<(), UperError> {
        const DET: usize = 1;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(DET)?;
        assert_eq!(buffer.content(), &[0x00 | DET as u8]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant()?);
        }

        assert_eq!(DET, buffer.read_length_determinant()?);
        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_127() -> Result<(), UperError> {
        const DET: usize = 126;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(DET)?;
        assert_eq!(buffer.content(), &[0x00 | DET as u8]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant()?);
        }

        assert_eq!(DET, buffer.read_length_determinant()?);
        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_128() -> Result<(), UperError> {
        const DET: usize = 128;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(DET)?;
        // detects that the value is greater than 127, so
        //   10xx_xxxx xxxx_xxxx (header)
        // | --00_0000 1000_0000 (128)
        // =======================
        //   1000_0000 1000_0000
        assert_eq!(buffer.content(), &[0x80 | 0x00, 0x00 | DET as u8]);

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant()?);
        }

        assert_eq!(DET, buffer.read_length_determinant()?);
        Ok(())
    }

    #[test]
    fn bit_buffer_length_determinant_16383() -> Result<(), UperError> {
        const DET: usize = 16383;
        let mut buffer = BitBuffer::default();
        buffer.write_length_determinant(DET)?;
        // detects that the value is greater than 127, so
        //   10xx_xxxx xxxx_xxxx (header)
        // | --11_1111 1111_1111 (16383)
        // =======================
        //   1011_1111 1111_1111
        assert_eq!(
            buffer.content(),
            &[0x80 | (DET >> 8) as u8, 0x00 | (DET & 0xFF) as u8]
        );

        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(DET, buffer2.read_length_determinant()?);
        }

        assert_eq!(DET, buffer.read_length_determinant()?);
        Ok(())
    }

    fn check_int_max(buffer: &mut BitBuffer, int: u64) -> Result<(), UperError> {
        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(int, buffer2.read_int_max()?);
        }

        assert_eq!(int, buffer.read_int_max()?);
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_0() -> Result<(), UperError> {
        const INT: u64 = 0;
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, INT as u8]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_127() -> Result<(), UperError> {
        const INT: u64 = 127; // u4::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, INT as u8]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_128() -> Result<(), UperError> {
        const INT: u64 = 128; // u4::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, INT as u8]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_255() -> Result<(), UperError> {
        const INT: u64 = 255; // u8::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 1 byte,
        // therefore the first byte is written
        // with 0x00 (header) | 1 (byte len).
        // The second byte is then the actual value
        assert_eq!(buffer.content(), &[0x00 | 1, INT as u8]);
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_256() -> Result<(), UperError> {
        const INT: u64 = 256; // u8::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 2 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 2 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 2,
                ((INT & 0xFF_00) >> 8) as u8,
                ((INT & 0x00_ff) >> 0) as u8,
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_65535() -> Result<(), UperError> {
        const INT: u64 = 65_535; // u16::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 2 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 2 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 2,
                ((INT & 0xFF_00) >> 8) as u8,
                ((INT & 0x00_FF) >> 0) as u8
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_65536() -> Result<(), UperError> {
        const INT: u64 = 65_536; // u16::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 3 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 3 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 3,
                ((INT & 0xFF_00_00) >> 16) as u8,
                ((INT & 0x00_FF_00) >> 8) as u8,
                ((INT & 0x00_00_FF) >> 0) as u8,
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_16777215() -> Result<(), UperError> {
        const INT: u64 = 16_777_215; // u24::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 3 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 3 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 3,
                ((INT & 0xFF_00_00) >> 16) as u8,
                ((INT & 0x00_FF_00) >> 8) as u8,
                ((INT & 0x00_00_FF) >> 0) as u8,
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_16777216() -> Result<(), UperError> {
        const INT: u64 = 16_777_216; // u24::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 4 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 4 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 4,
                ((INT & 0xFF_00_00_00) >> 24) as u8,
                ((INT & 0x00_FF_00_00) >> 16) as u8,
                ((INT & 0x00_00_FF_00) >> 8) as u8,
                ((INT & 0x00_00_00_FF) >> 0) as u8,
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_4294967295() -> Result<(), UperError> {
        const INT: u64 = 4_294_967_295; // u32::max_value() as u64
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 4 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 4 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 4,
                ((INT & 0xFF_00_00_00) >> 24) as u8,
                ((INT & 0x00_FF_00_00) >> 16) as u8,
                ((INT & 0x00_00_FF_00) >> 8) as u8,
                ((INT & 0x00_00_00_FF) >> 0) as u8,
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_4294967296() -> Result<(), UperError> {
        const INT: u64 = 4_294_967_296; // u32::max_value() as u64 + 1
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 5 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 5 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 5,
                ((INT & 0xFF_00_00_00_00) >> 32) as u8,
                ((INT & 0x00_FF_00_00_00) >> 24) as u8,
                ((INT & 0x00_00_FF_00_00) >> 16) as u8,
                ((INT & 0x00_00_00_FF_00) >> 8) as u8,
                ((INT & 0x00_00_00_00_FF) >> 0) as u8,
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_max_i64_max() -> Result<(), UperError> {
        const INT: u64 = 0x7F_FF_FF_FF_FF_FF_FF_FF_u64;
        assert_eq!(INT, i64::max_value() as u64);
        let mut buffer = BitBuffer::default();
        buffer.write_int_max(INT)?;
        // Can be represented in 8 bytes,
        // therefore the first byte is written
        // with 0x00 (header) | 8 (byte len).
        // The second byte is then the actual value
        assert_eq!(
            buffer.content(),
            &[
                0x00 | 8,
                ((INT & 0xFF_00_00_00_00_00_00_00) >> 56) as u8,
                ((INT & 0x00_FF_00_00_00_00_00_00) >> 48) as u8,
                ((INT & 0x00_00_FF_00_00_00_00_00) >> 40) as u8,
                ((INT & 0x00_00_00_FF_00_00_00_00) >> 32) as u8,
                ((INT & 0x00_00_00_00_FF_00_00_00) >> 24) as u8,
                ((INT & 0x00_00_00_00_00_FF_00_00) >> 16) as u8,
                ((INT & 0x00_00_00_00_00_00_FF_00) >> 8) as u8,
                ((INT & 0x00_00_00_00_00_00_00_FF) >> 0) as u8,
            ]
        );
        check_int_max(&mut buffer, INT)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_write_int_detects_not_in_range_positive_only() {
        let mut buffer = BitBuffer::default();
        // lower check
        assert_eq!(
            buffer.write_int(0, (10, 127)),
            Err(UperError::ValueNotInRange(0, 10, 127))
        );
        // upper check
        assert_eq!(
            buffer.write_int(128, (10, 127)),
            Err(UperError::ValueNotInRange(128, 10, 127))
        );
    }

    #[test]
    fn bit_buffer_write_int_detects_not_in_range_negative() {
        let mut buffer = BitBuffer::default();
        // lower check
        assert_eq!(
            buffer.write_int(-11, (-10, -1)),
            Err(UperError::ValueNotInRange(-11, -10, -1))
        );
        // upper check
        assert_eq!(
            buffer.write_int(0, (-10, -1)),
            Err(UperError::ValueNotInRange(0, -10, -1))
        );
    }

    #[test]
    fn bit_buffer_write_int_detects_not_in_range_with_negative() {
        let mut buffer = BitBuffer::default();
        // lower check
        assert_eq!(
            buffer.write_int(-11, (-10, 1)),
            Err(UperError::ValueNotInRange(-11, -10, 1))
        );
        // upper check
        assert_eq!(
            buffer.write_int(2, (-10, 1)),
            Err(UperError::ValueNotInRange(2, -10, 1))
        );
    }

    fn check_int(buffer: &mut BitBuffer, int: i64, range: (i64, i64)) -> Result<(), UperError> {
        {
            let mut buffer2 = BitBuffer::from_bits(buffer.content().into(), buffer.bit_len());
            assert_eq!(int, buffer2.read_int(range)?);
        }
        assert_eq!(int, buffer.read_int(range)?);
        Ok(())
    }

    #[test]
    fn bit_buffer_int_7bits() -> Result<(), UperError> {
        const INT: i64 = 10;
        const RANGE: (i64, i64) = (0, 127);
        let mut buffer = BitBuffer::default();
        buffer.write_int(INT, RANGE)?;
        // [0; 127] are 128 numbers, so they
        // have to fit in 7 bit
        assert_eq!(buffer.content(), &[(INT as u8) << 1]);
        check_int(&mut buffer, INT, RANGE)?;
        // be sure write_bit writes at the 8th bit
        buffer.write_bit(true)?;
        assert_eq!(buffer.content(), &[(INT as u8) << 1 | 0b0000_0001]);
        Ok(())
    }

    #[test]
    fn bit_buffer_int_neg() -> Result<(), UperError> {
        const INT: i64 = -10;
        const RANGE: (i64, i64) = (-128, 127);
        let mut buffer = BitBuffer::default();
        buffer.write_int(INT, RANGE)?;
        // [-128; 127] are 255 numbers, so they
        // have to fit in one byte
        assert_eq!(buffer.content(), &[(INT - RANGE.0) as u8]);
        check_int(&mut buffer, INT, RANGE)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_int_neg_extended_range() -> Result<(), UperError> {
        const INT: i64 = -10;
        const RANGE: (i64, i64) = (-128, 128);
        let mut buffer = BitBuffer::default();
        buffer.write_int(INT, RANGE)?;
        // [-128; 127] are 256 numbers, so they
        // don't fit in one byte but in 9 bits
        assert_eq!(
            buffer.content(),
            &[
                ((INT - RANGE.0) as u8) >> 1,
                (((INT - RANGE.0) as u8) << 7) | 0b0000_0000
            ]
        );
        // be sure write_bit writes at the 10th bit
        buffer.write_bit(true)?;
        assert_eq!(
            buffer.content(),
            &[
                ((INT - RANGE.0) as u8) >> 1,
                ((INT - RANGE.0) as u8) << 7 | 0b0100_0000
            ]
        );
        check_int(&mut buffer, INT, RANGE)?;
        Ok(())
    }

    #[test]
    fn bit_buffer_octet_string_with_range() -> Result<(), UperError> {
        // test scenario from https://github.com/alexvoronov/geonetworking/blob/57a43113aeabc25f005ea17f76409aed148e67b5/camdenm/src/test/java/net/gcdc/camdenm/UperEncoderDecodeTest.java#L169
        const BYTES: &[u8] = &[0x2A, 0x2B, 0x96, 0xFF];
        const RANGE: (i64, i64) = (1, 20);
        let mut buffer = BitBuffer::default();
        buffer.write_octet_string(BYTES, Some(RANGE))?;
        assert_eq!(&[0x19, 0x51, 0x5c, 0xb7, 0xf8], &buffer.content(),);
        Ok(())
    }

    #[test]
    fn bit_buffer_octet_string_without_range() -> Result<(), UperError> {
        const BYTES: &[u8] = &[0x2A, 0x2B, 0x96, 0xFF];
        let mut buffer = BitBuffer::default();
        buffer.write_octet_string(BYTES, None)?;
        assert_eq!(&[0x04, 0x2a, 0x2b, 0x96, 0xff], &buffer.content(),);
        Ok(())
    }

    #[test]
    fn bit_buffer_octet_string_empty() -> Result<(), UperError> {
        const BYTES: &[u8] = &[];
        let mut buffer = BitBuffer::default();
        buffer.write_octet_string(BYTES, None)?;
        assert_eq!(&[0x00], &buffer.content(),);
        Ok(())
    }
}
