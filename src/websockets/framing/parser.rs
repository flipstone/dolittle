use websockets::framing::types::*;

#[deriving(Eq,Clone)]
enum ParserState {
  INITIAL,
  AWAITING_BYTE_TWO,
  READING_PAYLOAD_LENGTH(Buffer),
  READING_MASK(Buffer),
  READING_PAYLOAD,
  DONE,
}

#[deriving(Eq,Clone)]
struct Buffer {
  bytes_read: u8,
  buf_value: u64,
}

#[deriving(Clone)]
pub struct FrameParser {
  state: ParserState,
  byte_one: Option<ByteOne>,
  byte_two: Option<ByteTwo>,
  payload_length: Option<PayloadLength>,
  masking_key: Option<MaskingKey>,
  payload_data: MaskedPayload,
}

pub struct ParseResult {
  parser: FrameParser,
  bytes_parsed: uint,
}

impl FrameParser {
  pub fn new() -> FrameParser {
    FrameParser {
      state: INITIAL,
      byte_one: None,
      byte_two: None,
      payload_length: None,
      masking_key: None,
      payload_data: MaskedPayload::new(),
    }
  }

  pub fn parse(&self, bytes: &[u8]) -> ParseResult {
    let initial = ParseResult {
      parser: self.clone(),
      bytes_parsed: 0,
    };

    let result = FrameParser::parse_bytewise(initial, bytes);

    if result.parser.state == READING_PAYLOAD {
      result.parser.consume_payload_bytes(result.bytes_parsed,
                                          bytes)
    } else {
      result
    }
  }

  fn parse_all(&self, bytes: &[u8]) -> FrameParser {
    let result = self.parse(bytes);

    assert!(result.bytes_parsed == bytes.len());

    result.parser
  }

  fn is_done(&self) -> bool {
    self.state == DONE
  }

  priv fn parse_bytewise(initial: ParseResult, bytes: &[u8]) -> ParseResult {
    let mut result = initial;

    for vec::each(bytes) |byte| {
      if result.parser.state == DONE ||
         result.parser.state == READING_PAYLOAD {
        break;
      } else {
        result = ParseResult {
          parser: result.parser.parse_byte(*byte),
          bytes_parsed: result.bytes_parsed + 1,
        }
      }
    }

    result
  }

  priv fn parse_byte(&self, byte: u8) -> FrameParser {
    match self.state {

      INITIAL => self.parse_byte_one(byte),
      AWAITING_BYTE_TWO => self.parse_byte_two(byte),
      READING_PAYLOAD_LENGTH(buf) => self.parse_payload_length_byte(buf, byte),
      READING_MASK(buf) => self.parse_mask_byte(buf, byte),
      READING_PAYLOAD => {
        error!("Attempt to read payload one byte at a time!");
        self.clone()
      }
      DONE => self.clone(),
    }
  }

  priv fn parse_byte_one(self, byte: u8) -> FrameParser {
    FrameParser {
      byte_one: Some(ByteOne(byte)),
      state: AWAITING_BYTE_TWO,
      ..
      self
    }
  }

  priv fn parse_byte_two(self, byte: u8) -> FrameParser {
    let byte_two = ByteTwo(byte);
    let payload_length = match byte_two.payload_length() {
      Length(len) => Some(len as u64),
      _ => None
    };

    FrameParser {
      byte_two: Some(byte_two),
      payload_length: payload_length,
      state: next_state_after_byte_two(byte_two),
      ..
      self
    }
  }

  priv fn parse_payload_length_byte(self, buf: Buffer, byte: u8) -> FrameParser {
    let new_buf = buf.add_byte(byte);
    let byte_two = self.byte_two.expect("Parsing payload without byte two!");
    let bytes_to_read = byte_two.payload_bytes_to_read();

    assert!(new_buf.bytes_read <= bytes_to_read);

    if new_buf.bytes_read == bytes_to_read {
      FrameParser {
        state: next_state_after_payload_length(byte_two),
        payload_length: Some(new_buf.buf_value),
        ..
        self
      }
    } else {
      FrameParser {
        state: READING_PAYLOAD_LENGTH(new_buf),
        ..
        self
      }
    }
  }

  priv fn parse_mask_byte(self, buf: Buffer, byte: u8) -> FrameParser {
    let new_buf = buf.add_byte(byte);
    let bytes_to_read = 4;

    assert!(new_buf.bytes_read <= bytes_to_read);

    if new_buf.bytes_read == bytes_to_read {
      FrameParser {
        masking_key: Some(MaskingKey(new_buf.buf_value as u32)),
        state: READING_PAYLOAD,
        ..
        self
      }
    } else {
      FrameParser {
        state: READING_MASK(new_buf),
        ..
        self
      }
    }
  }

  priv fn consume_payload_bytes(self, read_so_far: uint, bytes: &[u8]) -> ParseResult {
    let len_read_so_far = self.payload_data.length();
    let total_len = self.payload_length.expect("Got to parsing payload without length!") as uint;
    let len_left = cmp::max(0,total_len - len_read_so_far);

    let remaining_bytes = bytes.tailn(read_so_far);
    let len_to_read = cmp::min(len_left, remaining_bytes.len());
    let bytes_to_read = remaining_bytes.slice(0,len_to_read);

    ParseResult {
      parser: self.parse_payload_bytes(bytes_to_read),
      bytes_parsed: read_so_far + bytes_to_read.len(),
    }
  }

  priv fn parse_payload_bytes(self, bytes: &[u8]) -> FrameParser {
    let payload_data = self.payload_data.add_bytes(bytes);
    let max_length = self.payload_length.expect("Got to parsing payload without length!") as uint;

    assert!(payload_data.length() <= max_length);

    let state = if payload_data.length() == max_length {
      DONE
    } else {
      READING_PAYLOAD
    };

    FrameParser {
      state: state,
      payload_data: payload_data,
      ..
      self
    }
  }
}

fn next_state_after_byte_two(byte_two: ByteTwo) -> ParserState {
  if byte_two.is_extended_payload_length() {
    READING_PAYLOAD_LENGTH(Buffer::new())
  } else {
    next_state_after_payload_length(byte_two)
  }
}

fn next_state_after_payload_length(byte_two: ByteTwo) -> ParserState {
  if byte_two.is_mask() {
    READING_MASK(Buffer::new())
  } else {
    READING_PAYLOAD
  }
}

impl Buffer {
  fn new() -> Buffer {
    Buffer {
      bytes_read: 0,
      buf_value: 0,
    }
  }

  fn add_byte(&self, byte: u8) -> Buffer {
    Buffer {
      bytes_read: self.bytes_read + 1,
      buf_value: (self.buf_value << 8) | (byte as u64),
    }
  }
}

impl ParseResult {
  pub fn try_make_frame(&self) -> Option<Frame> {
    if self.parser.byte_one.is_none() {
      return None;
    }

    let byte_one = self.parser.byte_one.expect("is_none() lied about byte_one!");

    Some(Frame {
      fin: byte_one.is_fin(),
      reserved: byte_one.is_reserved(),
      op_code: byte_one.op_code(),
      masking_key: self.parser.masking_key,
      payload_data: self.parser.payload_data,
    })
  }

  pub fn make_frame_done(&self) -> Frame {
    assert!(self.is_done(), "ParseResult.make_frame_done called while not done!");
    self.try_make_frame().expect("ParseResult was done, but couldn't make frame!")
  }

  pub fn is_done(&self) -> bool {
    self.parser.is_done()
  }
}

#[test]
fn parse_zero_bytes() {
  let p = FrameParser::new().parse_all([]);

  assert!(p.byte_one == None);
  assert!(p.byte_two == None);
}

#[test]
fn parse_two_bytes_together() {
  let p = FrameParser::new()
          .parse_all([0x08, 0x0A]);

  assert!(p.byte_one == Some(ByteOne(0x08)));
  assert!(p.byte_two == Some(ByteTwo(0x0A)));
}

#[test]
fn parse_two_bytes_one_at_a_time() {
  let p = FrameParser::new()
          .parse_all([0x08])
          .parse_all([0x0A]);

  assert!(p.byte_one == Some(ByteOne(0x08)));
  assert!(p.byte_two == Some(ByteTwo(0x0A)));
}

#[test]
fn parse_short_payload_length() {
  let p = FrameParser::new()
          .parse_all([0x08])
          .parse_all([0x8A]);

  assert!(p.payload_length == Some(0x0A));
}

#[test]
fn parse_parse_two_byte_payload_length() {
  let p = FrameParser::new()
          .parse_all([0x00, 126, 0x7A, 0x4B]);

  assert!(p.payload_length == Some(0x7A4B));
}


#[test]
fn parse_parse_eight_byte_payload_length() {
  let p = FrameParser::new()
          .parse_all([0x00, 127,
                  0x7A,0x4B,0x64,0xF2,0xC4,0x42,0x99,0x88]);

  assert!(p.payload_length == Some(0x7A4B64F2C4429988));
}

#[test]
fn parse_mask_after_short_payload_length() {
  let p = FrameParser::new()
          .parse_all([0x00,0x80,0x7A,0x4B,0x64,0xF2]);

  assert!(p.masking_key == Some(MaskingKey(0x7A4B64F2)));
}

#[test]
fn parse_mask_after_two_byte_payload_length() {
  let p = FrameParser::new()
          .parse_all([0x00,0xFE,
                  0xCC,0xCC,
                  0x7A,0x4B,0x64,0xF2]);

  assert!(p.masking_key == Some(MaskingKey(0x7A4B64F2)));
}

#[test]
fn parse_mask_after_eight_byte_payload_length() {
  let p = FrameParser::new()
          .parse_all([0x00,0xFF,
                  0xCC,0xCC,0xCC,0xCC,0xCC,0xCC,0xCC,0xCC,
                  0x7A,0x4B,0x64,0xF2]);

  assert!(p.masking_key == Some(MaskingKey(0x7A4B64F2)));
}

#[test]
fn parse_payload_data_after_mask() {
  let p = FrameParser::new()
          .parse_all([0x00,0x85,
                  0x7A,0x4B,0x64,0xF2,
                  0x55,0x44,0x33,0x22,0x11]);

  assert!(p.payload_data == MaskedPayload(PayloadData(@[0x55,0x44,0x33,0x22,0x11])));
  assert!(p.is_done());
}

#[test]
fn parse_payload_data_without_mask() {
  let p = FrameParser::new()
          .parse_all([0x00,0x05,
                  0x55,0x44,0x33,0x22,0x11]);

  assert!(p.payload_data == MaskedPayload(PayloadData(@[0x55,0x44,0x33,0x22,0x11])));
  assert!(p.is_done());
}

#[test]
fn parse_with_extra_bytes() {
  let result = FrameParser::new()
               .parse([0x00,
                       0x01, // length 1
                       0x55, // data
                       0x44,0x33,0x22,0x11 // extra
                       ]);

  assert!(result.parser.is_done());
  assert!(result.bytes_parsed == 3);
}

#[test]
fn parse_with_no_bytes_parsed() {
  let result = FrameParser::new()
               .parse_all([0x00,
                           0x02, // length 1
                           0x55, 0x44]) // data
               .parse([0x44,0x33,0x22,0x11]); // extra

  assert!(result.bytes_parsed == 0);
}

#[test]
fn parse_payload_in_multiple_chunks() {
  let result = FrameParser::new()
               .parse_all([0x00,
                           0x0A, // length 10
                           0x01, 0x02]) // data
               .parse_all([3,4,5,6]) // more data
               .parse_all([7,8]) // more data
               .parse([9,10, // more data
                       0x22,0x11]); // extra

  assert!(result.bytes_parsed == 2);
  assert!(result.parser.payload_data ==
          MaskedPayload(PayloadData(@[1,2,3,4,5,6,7,8,9,10])));
}

