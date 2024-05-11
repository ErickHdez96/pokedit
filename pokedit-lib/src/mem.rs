pub mod le {
    #![allow(dead_code)]
    use std::io::Write;

    /// Writes `value` as little endian into `bytes` at `offset`.
    ///
    /// ```text
    /// bytes[offset] = value & 0xFF;
    /// bytes[offset + 1] = (value >> 8) & 0xFF;
    /// ```
    pub fn write_half_word(bytes: &mut [u8], offset: usize, value: u16) {
        (&mut bytes[offset..(offset + 2)])
            .write_all(&value.to_le_bytes())
            .unwrap();
    }

    pub fn read_half_word(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes(bytes[offset..(offset + 2)].try_into().unwrap())
    }

    pub fn write_word(bytes: &mut [u8], offset: usize, value: u32) {
        (&mut bytes[offset..(offset + 4)])
            .write_all(&value.to_le_bytes())
            .unwrap();
    }

    pub fn read_word(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..(offset + 4)].try_into().unwrap())
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn write_half_word() {
            let mut bytes = [0u8; 4];
            super::write_half_word(&mut bytes, 1, 0x1234);
            assert_eq!([0, 0x34, 0x12, 0], bytes);
        }

        #[test]
        fn read_half_word() {
            let mut bytes = [0, 0x34, 0x12, 0];
            assert_eq!(0x1234, super::read_half_word(&mut bytes, 1));
        }

        #[test]
        fn write_word() {
            let mut bytes = [0u8; 6];
            super::write_word(&mut bytes, 1, 0x12345678);
            assert_eq!([0, 0x78, 0x56, 0x34, 0x12, 0], bytes);
        }

        #[test]
        fn read_word() {
            let mut bytes = [0, 0x78, 0x56, 0x34, 0x12, 0];
            assert_eq!(0x12345678, super::read_word(&mut bytes, 1));
        }
    }
}

pub mod be {
    #![allow(dead_code)]
    use std::io::Write;

    pub fn write_half_word(bytes: &mut [u8], offset: usize, value: u16) {
        (&mut bytes[offset..(offset + 2)])
            .write_all(&value.to_be_bytes())
            .unwrap();
    }

    pub fn read_half_word(bytes: &[u8], offset: usize) -> u16 {
        u16::from_be_bytes(bytes[offset..(offset + 2)].try_into().unwrap())
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn write_half_word() {
            let mut bytes = [0u8; 4];
            super::write_half_word(&mut bytes, 1, 0x1234);
            assert_eq!([0, 0x12, 0x34, 0], bytes);
        }

        #[test]
        fn read_half_word() {
            let mut bytes = [0, 0x12, 0x34, 0];
            assert_eq!(0x1234, super::read_half_word(&mut bytes, 1));
        }
    }
}
