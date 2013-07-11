use websockets::messaging::{Fragment,FragmentType,Text,Data,Continuation};

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

impl Fragment for Frame {
  fn fragment_type(&self) -> FragmentType {
    match self.op_code {
      CONTINUATION => Continuation,
      TEXT => Text,
      BINARY => Data,
      _ => fail!(~"Invalid opcode for fragmenting: " +
                 self.op_code.to_byte().to_str() + ~"!")
    }
  }

  fn fragment_bytes(&self) -> @[u8] {
    self.unmasked_payload().to_managed_bytes()
  }

  fn is_fin(&self) -> bool {
    self.is_fin()
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
    OpCode::from_byte(**self & OP_CODE_MASK)
  }
}

#[deriving(Eq,Clone)]
pub enum OpCode {
  CONTINUATION,
  TEXT,
  BINARY,
  RESERVED_NON_CONTROL(u8),
  CONNECTION_CLOSE,
  PING,
  PONG,
  RESERVED_CONTROL(u8),
}

impl OpCode {
  pub fn from_byte(byte: u8) -> OpCode {
    match byte {
      0x0 => CONTINUATION,
      0x1 => TEXT,
      0x2 => BINARY,
      0x3..0x7 => RESERVED_NON_CONTROL(byte),
      0x8 => CONNECTION_CLOSE,
      0x9 => PING,
      0xA => PONG,
      _ => RESERVED_CONTROL(byte)
    }
  }

  pub fn to_byte(&self) -> u8 {
    match *self {
      CONTINUATION => 0x0,
      TEXT => 0x1,
      BINARY => 0x2,
      RESERVED_NON_CONTROL(byte) => byte,
      CONNECTION_CLOSE => 0x8,
      PING => 0x9,
      PONG => 0xA,
      RESERVED_CONTROL(byte) => byte,
    }
  }

  pub fn is_control(&self) -> bool {
    (self.to_byte() & 0x8) == 0x8
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

  fn to_managed_bytes(&self) -> @[u8] {
    **self
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
  assert!(ByteOne(0xFF).op_code() == RESERVED_CONTROL(0xF));
}

#[test]
fn opcode_decoding() {
  assert!(OpCode::from_byte(0x0) == CONTINUATION);
  assert!(OpCode::from_byte(0x1) == TEXT);
  assert!(OpCode::from_byte(0x2) == BINARY);
  assert!(OpCode::from_byte(0x3) == RESERVED_NON_CONTROL(0x3));
  assert!(OpCode::from_byte(0x4) == RESERVED_NON_CONTROL(0x4));
  assert!(OpCode::from_byte(0x5) == RESERVED_NON_CONTROL(0x5));
  assert!(OpCode::from_byte(0x6) == RESERVED_NON_CONTROL(0x6));
  assert!(OpCode::from_byte(0x7) == RESERVED_NON_CONTROL(0x7));
  assert!(OpCode::from_byte(0x8) == CONNECTION_CLOSE);
  assert!(OpCode::from_byte(0x9) == PING);
  assert!(OpCode::from_byte(0xA) == PONG);
  assert!(OpCode::from_byte(0xB) == RESERVED_CONTROL(0xB));
  assert!(OpCode::from_byte(0xC) == RESERVED_CONTROL(0xC));
  assert!(OpCode::from_byte(0xD) == RESERVED_CONTROL(0xD));
  assert!(OpCode::from_byte(0xE) == RESERVED_CONTROL(0xE));
  assert!(OpCode::from_byte(0xF) == RESERVED_CONTROL(0xF));
}


fn non_control_opcode_bytes() -> ~[u8] {
  ~[0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7]
}

fn control_opcode_bytes() -> ~[u8] {
  ~[0x8, 0x9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF]
}

fn all_opcode_bytes() -> ~[u8] {
  non_control_opcode_bytes() + control_opcode_bytes()
}

#[test]
fn opcode_is_control() {
  for non_control_opcode_bytes().each() |byte| {
    assert!(!OpCode::from_byte(*byte).is_control());
  }

  for control_opcode_bytes().each() |byte| {
    assert!(OpCode::from_byte(*byte).is_control());
  }
}

#[test]
fn opcode_to_byte() {
  for all_opcode_bytes().each() |byte| {
    assert!(OpCode::from_byte(*byte).to_byte() == *byte);
  }
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
