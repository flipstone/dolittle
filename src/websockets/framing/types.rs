#[deriving(Eq,Clone)]
pub struct Frame {
  fin: bool,
  reserved: bool,
  op_code: OpCode,
  masking_key: Option<MaskingKey>,
  payload_data: MaskedPayload,
}

impl Frame {
  pub fn unmasked_payload(&self) -> PayloadData {
    self.payload_data.unmask(self.masking_key)
  }

  pub fn is_fin(&self) -> bool {
    self.fin
  }

  pub fn is_reserved(&self) -> bool {
    self.reserved
  }
}

#[deriving(Eq,Clone)]
pub struct ByteOne(u8);

#[deriving(Eq,Clone)]
pub struct ByteTwo(u8);

pub static FIN_MASK: u8 = 0x80;
pub static RESERVED_MASK: u8 = 0x70;
pub static OP_CODE_MASK: u8 = 0x0F;
pub static MASK_MASK: u8 = 0x80;
pub static PAYLOAD_LENGTH_MASK: u8 = 0x7F;

impl ByteOne {
  fn is_fin(&self) -> bool {
    (**self) & FIN_MASK != 0
  }

  fn is_reserved(&self) -> bool {
    (**self) & RESERVED_MASK != 0
  }

  fn op_code(&self) -> OpCode {
    OpCode::from_byte(**self)
  }
}

#[deriving(Eq,Clone)]
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

impl OpCode {
  pub fn from_byte(byte: u8) -> OpCode {
    match byte & OP_CODE_MASK {
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

  pub fn to_byte(&self) -> u8{
    match *self {
      CONTINUATION => 0x0,
      TEXT => 0x1,
      BINARY => 0x2,
      CONNECTION_CLOSE => 0x8,
      PING => 0x9,
      PONG => 0xA,
      RESERVED_NON_CONTROL => {
        error!("Attempt to encode RESERVED_NON_CONTROL opcode");
        0x00
      }
      RESERVED_CONTROL => {
        error!("Attempt to encode RESERVED_CONTROL opcode");
        0x00
      }
    }
  }
}

impl ByteTwo {
  fn is_mask(&self) -> bool {
    (**self) & MASK_MASK != 0
  }

  fn payload_length(&self) -> ByteTwoPayloadLength {
    match (**self & PAYLOAD_LENGTH_MASK) {
      127 => NextEightBytesAreLength,
      126 => NextTwoBytesAreLength,
      len => Length(len)
    }
  }

  fn is_extended_payload_length(&self) -> bool {
    (**self & PAYLOAD_LENGTH_MASK) > 125
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

  fn apply(&self, payload: PayloadData) -> PayloadData {
    let bytes = do vec::mapi(*payload) |idx, byte| {
      self.byte_mask(idx) ^ *byte
    };

    PayloadData::from_bytes(bytes)
  }

  fn to_bytes(&self) -> ~[u8] {
    ~[
      self.byte_mask(0),
      self.byte_mask(1),
      self.byte_mask(2),
      self.byte_mask(3),
     ]
  }
}

#[deriving(Eq,Clone)]
pub struct MaskedPayload(PayloadData);

impl MaskedPayload {
  pub fn new() -> MaskedPayload {
    MaskedPayload(PayloadData::new())
  }

  fn unmask(&self, key: Option<MaskingKey>) -> PayloadData {
    match key {
      Some(key) => key.apply(**self),
      None => **self
    }
  }

  fn add_bytes(&self, bytes: &[u8]) -> MaskedPayload {
    MaskedPayload((**self).add_bytes(bytes))
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

  pub fn from_bytes(bytes: &[u8]) -> PayloadData {
    PayloadData::new().add_bytes(bytes)
  }

  fn mask(&self, key: Option<MaskingKey>) -> MaskedPayload {
    match key {
      Some(key) => MaskedPayload(key.apply(*self)),
      None => MaskedPayload(*self)
    }
  }

  fn to_bytes(&self) -> ~[u8] {
    (**self).to_owned()
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
}

#[test]
fn opcode_decoding() {
  assert!(OpCode::from_byte(0x00) == CONTINUATION);
  assert!(OpCode::from_byte(0x01) == TEXT);
  assert!(OpCode::from_byte(0x02) == BINARY);
  assert!(OpCode::from_byte(0x03) == RESERVED_NON_CONTROL);
  assert!(OpCode::from_byte(0x04) == RESERVED_NON_CONTROL);
  assert!(OpCode::from_byte(0x05) == RESERVED_NON_CONTROL);
  assert!(OpCode::from_byte(0x06) == RESERVED_NON_CONTROL);
  assert!(OpCode::from_byte(0x07) == RESERVED_NON_CONTROL);
  assert!(OpCode::from_byte(0x08) == CONNECTION_CLOSE);
  assert!(OpCode::from_byte(0x09) == PING);
  assert!(OpCode::from_byte(0x0A) == PONG);
  assert!(OpCode::from_byte(0x0B) == RESERVED_CONTROL);
  assert!(OpCode::from_byte(0x0C) == RESERVED_CONTROL);
  assert!(OpCode::from_byte(0x0D) == RESERVED_CONTROL);
  assert!(OpCode::from_byte(0x0E) == RESERVED_CONTROL);
  assert!(OpCode::from_byte(0x0F) == RESERVED_CONTROL);
}

#[test]
fn opcode_encoding() {
  assert!(OpCode::from_byte(CONTINUATION.to_byte()) == CONTINUATION);
  assert!(OpCode::from_byte(TEXT.to_byte()) == TEXT);
  assert!(OpCode::from_byte(BINARY.to_byte()) == BINARY);
  assert!(OpCode::from_byte(CONNECTION_CLOSE.to_byte()) == CONNECTION_CLOSE);
  assert!(OpCode::from_byte(PING.to_byte()) == PING);
  assert!(OpCode::from_byte(PONG.to_byte()) == PONG);
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
fn masking_key_apply() {
  let key = MaskingKey(0xFFF00F00);
  let masked = key.apply(PayloadData(@[0x0F,0xF0,0x0F,0xF0,
                                       0xF0,0xFF]));

  assert!(masked == PayloadData(@[0xF0,0x00,0x00,0xF0,0x0F,0x0F]));
}

#[test]
fn masking_key_to_bytes() {
  let key = MaskingKey(0xFFF00F00);
  assert!(key.to_bytes() == ~[0xFF,0xF0,0x0F,0x00])
}
