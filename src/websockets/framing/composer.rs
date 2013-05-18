use websockets::framing::types::*;
use websockets::framing::parser::*;

fn mask_if(test: bool, mask: u8) -> u8 {
  if test {
    mask
  } else {
    0x00
  }
}

impl Frame {
  pub fn compose(&self) -> ~[u8] {
    let length = self.payload_data.length();

    ~[self.compose_byte_one(),
      self.compose_byte_two(length)] +
     self.compose_payload_length(length) +
     self.compose_mask_bytes() +
     self.payload_data.to_bytes()
  }

  fn compose_byte_one(&self) -> u8 {
    let fin = mask_if(self.is_fin(), FIN_MASK);
    let resv = mask_if(self.is_reserved(), RESERVED_MASK);

    fin | resv | self.op_code.to_byte()
  }

  fn compose_byte_two(&self, length: uint) -> u8 {
    let mask = mask_if(self.masking_key.is_some(), MASK_MASK);

    let len =
      if length < 126 {
        (length as u8) & PAYLOAD_LENGTH_MASK
      } else if length <= 0xFFFF {
        126
      } else {
        127
      };

    mask | len
  }

  fn compose_payload_length(&self, length: uint) -> ~[u8] {
    if length < 126 {
      ~[]
    } else if length <= 0xFFFF {
      ~[(length >> 8) as u8, length as u8]
    } else {
      ~[
        (length >> 56) as u8,
        (length >> 48) as u8,
        (length >> 40) as u8,
        (length >> 32) as u8,
        (length >> 24) as u8,
        (length >> 16) as u8,
        (length >> 8) as u8,
        length as u8
       ]
    }
  }

  fn compose_mask_bytes(&self) -> ~[u8] {
    self.masking_key.map_default(~[],|key| key.to_bytes())
  }
}

#[test]
fn compose_base_case_1() {
  let frame = Frame {
    fin: true,
    reserved: true,
    op_code: TEXT,
    masking_key: None,
    payload_data: MaskedPayload(PayloadData(@[0x00,0x12])),
  };

  let parser = FrameParser::new();
  let result = parser.parse(frame.compose());

  assert!(result.make_frame_done() == frame);
}

#[test]
fn compose_base_case_2() {
  let frame = Frame {
    fin: false,
    reserved: false,
    op_code: BINARY,
    masking_key: Some(MaskingKey(0xFFFFFFFF)),
    payload_data: MaskedPayload(PayloadData(@[0x00,0x12,0x13])),
  };

  let parser = FrameParser::new();
  let result = parser.parse(frame.compose());

  assert!(result.make_frame_done() == frame);
}

#[test]
fn compose_with_long_payloads() {
  test_compose_long_payload(126);
  test_compose_long_payload(127);
  test_compose_long_payload(0x0100);
  test_compose_long_payload(0x010000);
}

fn test_compose_long_payload(length: uint) {
  let data = vec::from_elem(length, 0x0F);

  let frame = Frame {
    fin: false,
    reserved: false,
    op_code: BINARY,
    masking_key: Some(MaskingKey(0xFFFFFFFF)),
    payload_data: MaskedPayload(PayloadData::from_bytes(data)),
  };

  let parser = FrameParser::new();
  let result = parser.parse(frame.compose());

  assert!(result.make_frame_done() == frame);
}

