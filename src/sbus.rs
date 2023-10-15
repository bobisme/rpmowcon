const PACKET_PADDING_SIZE: usize = 0;
const PACKET_HEADER_SIZE: usize = 1;
const PACKET_FOOTER_SIZE: usize = 1;
const PACKET_DATA_SIZE: usize = 23;
const PACKET_SIZE: usize =
    PACKET_PADDING_SIZE + PACKET_HEADER_SIZE + PACKET_DATA_SIZE + PACKET_FOOTER_SIZE;
const BIT_MASK: u16 = (1 << 11) - 1;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, defmt::Format, Default)]
pub struct Chan(u16);

impl Chan {
    pub const fn new(val: u16) -> Self {
        let val = val & BIT_MASK;
        // let val = val.reverse_bits() >> 5;
        Self(val)
    }

    pub const fn get(&self) -> u16 {
        self.0
    }
}

const _: () = {
    // Chan truncates to 11 bits
    debug_assert!(Chan::new(0xFFFF).0 == 0b0000_0111_1111_1111);
    debug_assert!(Chan::new(0xFFFF).0 == 2_047);
};

pub const fn chan(val: u16) -> Chan {
    Chan::new(val)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, defmt::Format, Default)]
pub struct Data {
    pub channels: [Chan; 16],
    pub ch_17: bool,
    pub ch_18: bool,
    pub frame_lost: bool,
    pub failsafe: bool,
}

impl Data {
    pub const fn from_packet(packet: &[u8; PACKET_SIZE]) -> Self {
        let flag_byte = packet[PACKET_SIZE - 2];
        Self {
            channels: read_chans(packet.as_slice()),
            ch_17: (flag_byte & 0b1) > 0,
            ch_18: (flag_byte & 0b10) > 0,
            frame_lost: (flag_byte & 0b100) > 0,
            failsafe: (flag_byte & 0b1000) > 0,
        }
    }
}

#[inline]
const fn idx(i: usize, packet: &[u8]) -> u16 {
    packet[PACKET_PADDING_SIZE + PACKET_HEADER_SIZE + i] as u16
}

#[inline]
const fn idx11(i: usize, packet: &[u8]) -> u16 {
    packet[PACKET_PADDING_SIZE + PACKET_HEADER_SIZE + 11 + i] as u16
}

pub const fn read_chans(packet: &[u8]) -> [Chan; 16] {
    let p = packet;
    [
        chan(idx(1, p) << 8 | idx(0, p)),
        chan(idx(2, p) << 5 | idx(1, p) >> 3),
        chan(idx(4, p) << 10 | idx(3, p) << 2 | idx(2, p) >> 6),
        chan(idx(5, p) << 7 | idx(4, p) >> 1),
        chan(idx(6, p) << 4 | idx(5, p) >> 4),
        chan(idx(8, p) << 9 | idx(7, p) << 1 | idx(6, p) >> 7),
        chan(idx(9, p) << 6 | idx(8, p) >> 2),
        chan(idx(10, p) << 3 | idx(9, p) >> 5),
        // same ops as above, but offset by 11 bytes
        chan(idx11(1, p) << 8 | idx11(0, p)),
        chan(idx11(2, p) << 5 | idx11(1, p) >> 3),
        chan(idx11(4, p) << 10 | idx11(3, p) << 2 | idx11(2, p) >> 6),
        chan(idx11(5, p) << 7 | idx11(4, p) >> 1),
        chan(idx11(6, p) << 4 | idx11(5, p) >> 4),
        chan(idx11(8, p) << 9 | idx11(7, p) << 1 | idx11(6, p) >> 7),
        chan(idx11(9, p) << 6 | idx11(8, p) >> 2),
        chan(idx11(10, p) << 3 | idx11(9, p) >> 5),
    ]
}

// bit groupings are illustrative of 11-bit packed ints
#[allow(clippy::unusual_byte_groupings)]
const _: () = {
    let d: [u8; PACKET_SIZE] = [
        0u8,
        0b_00000001, // 0
        0b00010_000, // 1
        0b11_000000, // 2
        0b00000000,
        0b0000100_0, // 3
        0b0101_0000, // 4
        0b0_0000000, // 5
        0b00000011,
        0b000111_00, // 6
        0b000_00000, // 7
        0b00000001_,
        0b_00001001, // 8
        0b01010_000, // 9
        0b11_000000, // 10
        0b00000010,
        0b0001100_0, // 11
        0b1101_0000, // 12
        0b0_0000000, // 13
        0b00000111,
        0b001111_00, // 14
        0b000_00000, // 15
        0b00000010_,
        0,
        0,
    ];

    let actual = read_chans(&d);
    debug_assert!(actual[0].0 == 1);
    debug_assert!(actual[1].0 == 2);
    debug_assert!(actual[2].0 == 3);
    debug_assert!(actual[3].0 == 4);
    debug_assert!(actual[4].0 == 5);
    debug_assert!(actual[5].0 == 6);
    debug_assert!(actual[6].0 == 7);
    debug_assert!(actual[7].0 == 8);
    debug_assert!(actual[8].0 == 9);
    debug_assert!(actual[9].0 == 10);
    debug_assert!(actual[10].0 == 11);
    debug_assert!(actual[11].0 == 12);
    debug_assert!(actual[12].0 == 13);
    debug_assert!(actual[13].0 == 14);
    debug_assert!(actual[14].0 == 15);
    debug_assert!(actual[15].0 == 16);

    const fn new_packet(flag_byte: u8) -> [u8; PACKET_SIZE] {
        let mut packet = [0u8; PACKET_SIZE];
        packet[PACKET_SIZE - 2] = flag_byte;
        packet
    }

    assert!(!Data::from_packet(&new_packet(0b1110)).ch_17);
    assert!(Data::from_packet(&new_packet(0b0001)).ch_17);
    assert!(!Data::from_packet(&new_packet(0b1101)).ch_18);
    assert!(Data::from_packet(&new_packet(0b0010)).ch_18);
    assert!(!Data::from_packet(&new_packet(0b1011)).frame_lost);
    assert!(Data::from_packet(&new_packet(0b0100)).frame_lost);
    assert!(!Data::from_packet(&new_packet(0b0111)).failsafe);
    assert!(Data::from_packet(&new_packet(0b1000)).failsafe);
};

#[derive(Debug)]
pub struct Receiver {
    packet: [u8; PACKET_SIZE],
    size: usize,
}

impl Receiver {
    pub const fn new() -> Self {
        Self {
            packet: [0u8; PACKET_SIZE],
            size: 0,
        }
    }

    pub fn free_buf(&mut self) -> &mut [u8] {
        &mut self.packet[self.size..]
    }

    pub fn read_bytes(&mut self, count: usize) {
        self.size += count
    }

    pub fn reset(&mut self) {
        self.packet.fill(0);
        self.size = 0;
    }

    pub fn get_data(&mut self) -> Option<Data> {
        if self.packet[0] != 0x0F {
            defmt::error!("packet header does not match");
            self.reset();
            return None;
        }
        if self.size < PACKET_SIZE {
            return None;
        }
        if self.packet[PACKET_SIZE - 1] != 0x00 {
            defmt::error!("packet footer does not match");
            self.reset();
            return None;
        }
        let data = Data::from_packet(&self.packet);
        self.reset();
        Some(data)
    }
}
