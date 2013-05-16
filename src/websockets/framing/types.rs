#[deriving(Eq,Clone)]
pub struct Frame {
  byte_one: ByteOne,
  byte_two: ByteTwo,
  masking_key: Option<MaskingKey>,
  payload_data: PayloadData
}

impl Frame {
  pub fn unmasked_payload(&self) -> PayloadData {
    match self.masking_key {
      Some(key) => key.unmask(self.payload_data),
      None => self.payload_data
    }
  }

  pub fn is_fin(&self) -> bool {
    self.byte_one.is_fin()
  }
}

#[deriving(Eq,Clone)]
pub struct ByteOne(u8);

#[deriving(Eq,Clone)]
pub struct ByteTwo(u8);

impl ByteOne {
  fn is_fin(&self) -> bool {
    (**self) & 0x80 != 0
  }

  fn is_reserved(&self) -> bool {
    (**self) & 0x70 != 0
  }

  fn op_code(&self) -> OpCode {
    match (**self) & 0x0F {
      0x0 => CONTINUATION,
      0x1 => TEXT,
      0x2 => BINARY,
      0x3..0x7 => RESERVED_NON_CONTROL,
      0x8 => CONNECTION_CLOSE,
      0x9 => PING,
      0xA => PONG,
      _ => RESERVED_CONTROL
    }
  }
}

#[deriving(Eq)]
pub enum OpCode {
  CONTINUATION,
  TEXT,
  BINARY,
  RESERVED_NON_CONTROL,
  CONNECTION_CLOSE,
  PING,
  PONG,
  RESERVED_CONTROL,
}


impl ByteTwo {
  fn is_mask(&self) -> bool {
    (**self) & 0x80 != 0
  }

  fn payload_length(&self) -> ByteTwoPayloadLength {
    match (**self & 0x7F) {
      127 => NextEightBytesAreLength,
      126 => NextTwoBytesAreLength,
      len => Length(len)
    }
  }

  fn is_extended_payload_length(&self) -> bool {
    (**self & 0x7F) > 125
  }

  fn payload_bytes_to_read(&self) -> u8 {
    match self.payload_length() {
      NextEightBytesAreLength => 8,
      NextTwoBytesAreLength => 2,
      _ => 0,
    }
  }
}

pub type PayloadLength = u64;

#[deriving(Eq,Clone)]
pub struct MaskingKey(u32);

impl MaskingKey {
  fn byte_mask(&self, index: uint) -> u8 {
    let shift = 8*(3 - (index % 4));
    ((**self >> shift) & 0xFF) as u8
  }

  fn unmask(&self, payload: PayloadData) -> PayloadData {
    let bytes = do vec::mapi(*payload) |idx, byte| {
      self.byte_mask(idx) ^ *byte
    };

    PayloadData::new().add_bytes(bytes)
  }
}

#[deriving(Eq)]
pub struct PayloadData(@[u8]);

impl Clone for PayloadData {
  fn clone(&self) -> PayloadData {
    PayloadData(**self)
  }
}

impl Add<PayloadData,PayloadData> for PayloadData {
  fn add(&self, other: &PayloadData) -> PayloadData {
    self.add_bytes(**other)
  }
}

impl PayloadData {
  pub fn new() -> PayloadData {
    PayloadData(@[])
  }

  fn add_bytes(&self, bytes: &[u8]) -> PayloadData {
    PayloadData((**self) + bytes)
  }

  fn length(&self) -> uint {
    (**self).len()
  }
}

#[deriving(Eq)]
pub enum ByteTwoPayloadLength {
  Length(u8),
  NextTwoBytesAreLength,
  NextEightBytesAreLength
}

#[test]
fn frame_byte_1_bit_0_is_fin() {
  assert!(ByteOne(0x80).is_fin());
  assert!(!ByteOne(0x00).is_fin());
  assert!(!ByteOne(0x40).is_fin());
}

#[test]
fn frame_byte_1_bits_123_is_reserved() {
  assert!(ByteOne(0x40).is_reserved());
  assert!(ByteOne(0x30).is_reserved());
  assert!(ByteOne(0x10).is_reserved());
  assert!(!ByteOne(0x00).is_reserved());
  assert!(!ByteOne(0x80).is_reserved());
}

#[test]
fn frame_byte_1_bits_5678_is_opcode() {
  assert!(ByteOne(0x00).op_code() == CONTINUATION);
  assert!(ByteOne(0xF0).op_code() == CONTINUATION);
  assert!(ByteOne(0x01).op_code() == TEXT);
  assert!(ByteOne(0x02).op_code() == BINARY);
  assert!(ByteOne(0x03).op_code() == RESERVED_NON_CONTROL);
  assert!(ByteOne(0x04).op_code() == RESERVED_NON_CONTROL);
  assert!(ByteOne(0x05).op_code() == RESERVED_NON_CONTROL);
  assert!(ByteOne(0x06).op_code() == RESERVED_NON_CONTROL);
  assert!(ByteOne(0x07).op_code() == RESERVED_NON_CONTROL);
  assert!(ByteOne(0x08).op_code() == CONNECTION_CLOSE);
  assert!(ByteOne(0x09).op_code() == PING);
  assert!(ByteOne(0x0A).op_code() == PONG);
  assert!(ByteOne(0x0B).op_code() == RESERVED_CONTROL);
  assert!(ByteOne(0x0C).op_code() == RESERVED_CONTROL);
  assert!(ByteOne(0x0D).op_code() == RESERVED_CONTROL);
  assert!(ByteOne(0x0E).op_code() == RESERVED_CONTROL);
  assert!(ByteOne(0x0F).op_code() == RESERVED_CONTROL);
}


#[test]
fn frame_byte_2_bit_0_is_mask() {
  assert!(ByteTwo(0x80).is_mask());
  assert!(!ByteTwo(0x00).is_mask());
  assert!(!ByteTwo(0x40).is_mask());
}

#[test]
fn frame_byte_2_bits_1234567_payload_length() {
  assert!(ByteTwo(0x00).payload_length() == Length(0));
  assert!(ByteTwo(0x80).payload_length() == Length(0));
  assert!(ByteTwo(1).payload_length() == Length(1));
  assert!(ByteTwo(125).payload_length() == Length(125));
  assert!(ByteTwo(126).payload_length() == NextTwoBytesAreLength);
  assert!(ByteTwo(127).payload_length() == NextEightBytesAreLength);
}

#[test]
fn frame_byte_2_bits_1234567_payload_bytes_to_read() {
  assert!(ByteTwo(0x00).payload_bytes_to_read() == 0);
  assert!(ByteTwo(126).payload_bytes_to_read() == 2);
  assert!(ByteTwo(127).payload_bytes_to_read() == 8);
}


#[test]
fn frame_byte_2_bits_1234567_is_extended_payload_length() {
  assert!(!ByteTwo(0x00).is_extended_payload_length());
  assert!(!ByteTwo(125).is_extended_payload_length());
  assert!(!ByteTwo(0x80).is_extended_payload_length());
  assert!(ByteTwo(126).is_extended_payload_length());
  assert!(ByteTwo(127).is_extended_payload_length());
}

#[test]
fn masking_key_byte_mask() {
  let key = MaskingKey(0xFFF00F00);

  assert!(key.byte_mask(0) == 0xFF);
  assert!(key.byte_mask(1) == 0xF0);
  assert!(key.byte_mask(2) == 0x0F);
  assert!(key.byte_mask(3) == 0x00);
  assert!(key.byte_mask(4) == 0xFF);
}

#[test]
fn masking_key_unmask() {
  let key = MaskingKey(0xFFF00F00);
  let unmasked = key.unmask(PayloadData(@[0x0F,0xF0,0x0F,0xF0,
                                          0xF0,0xFF]));

  assert!(unmasked == PayloadData(@[0xF0,0x00,0x00,0xF0,0x0F,0x0F]));
}
