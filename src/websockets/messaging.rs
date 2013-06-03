#[deriving(Eq)]
enum Receiver {
  Unstarted,
  InProgress(FragmentType, DataSoFar),
}

#[deriving(Eq)]
enum Reception {
  Received(Either<DataMessage,TextMessage>),
  Receiving(Receiver),
  ReceptionError(ReceptionError),
}

type DataSoFar = @[u8];

#[deriving(Eq)]
struct DataMessage(@[u8]);
#[deriving(Eq)]
struct TextMessage(@str);

#[deriving(Eq)]
enum ReceptionError {
  CONTINUATION_AS_FIRST_FRAME,
  INVALID_MESSAGE_TYPE(FragmentType),
}

#[deriving(Eq)]
enum FragmentType {
  Text,
  Data,
  Continuation
}

trait Fragment {
  fn fragment_type(&self) -> FragmentType;
  fn fragment_bytes(&self) -> @[u8];
  fn is_fin(&self) -> bool;
}

impl Receiver {
  fn new() -> Receiver { Unstarted }

  fn next_fragment<F: Fragment>(&self, fragment: F) -> Reception {
    let (msg_type, message_so_far) =
      match *self {
        InProgress(t,msg) => (t,msg),
        Unstarted => (fragment.fragment_type(), @[]),
      };

    if msg_type == Continuation {
      ReceptionError(CONTINUATION_AS_FIRST_FRAME)
    } else {
      handle_fragment(msg_type, message_so_far, fragment)
    }
  }
}

fn handle_fragment<F: Fragment>(msg_type: FragmentType,
                                message_so_far: DataSoFar,
                                fragment: F) -> Reception {
  let message = message_so_far + fragment.fragment_bytes();

  if fragment.is_fin() {
    build_message_reception(msg_type, message)
  } else {
    Receiving(InProgress(msg_type,message))
  }
}

fn build_message_reception(msg_type: FragmentType, message: @[u8]) -> Reception {
  match msg_type {
    Data => Received(Left(DataMessage(message))),
    Text => {
      let text = str::from_bytes(message).to_managed();
      Received(Right(TextMessage(text)))
    },
    t => ReceptionError(INVALID_MESSAGE_TYPE(t)),
  }
}

#[test]
fn test_assemble_data_message_in_one_fragment() {
  let result = Receiver::new()
               .next_fragment((Data,true,@[0,1,2 as u8]));

  assert!(result == Received(Left(DataMessage(@[0,1,2]))));
}

#[test]
fn test_assemble_data_message_in_multiple_fragments() {
  let receiver = assert_receiving(
                  Receiver::new()
                  .next_fragment((Data,false,@[0,1,2 as u8])));

  let result = receiver.next_fragment((Continuation,true,@[3,4 as u8]));
  assert!(result == Received(Left(DataMessage(@[0,1,2,3,4]))));
}

#[test]
fn test_assemble_text_message_in_one_fragment() {
  let data = @[105, 32, 226, 153, 165, 32, 117 as u8];
  let result = Receiver::new().next_fragment((Text,true,data));

  assert!(result == Received(Right(TextMessage(@"i ♥ u"))));
}

#[test]
fn test_assemble_text_message_in_multiple_fragment() {
  let receiver = assert_receiving(
                  Receiver::new()
                  .next_fragment((Text,false,@[105,32,226 as u8])));

  let result = receiver.next_fragment((Continuation,true,@[153,165,32,117 as u8]));
  assert!(result == Received(Right(TextMessage(@"i ♥ u"))));
}

#[test]
fn test_error_when_initial_frame_is_continuation() {
  let result = Receiver::new()
               .next_fragment((Continuation,false,@[0 as u8]));

  assert!(result == ReceptionError(CONTINUATION_AS_FIRST_FRAME));
}

impl Fragment for (FragmentType,bool,@[u8]) {
  fn fragment_type(&self) -> FragmentType {
    match *self { (fragment_type,_,_) => fragment_type }
  }

  fn is_fin(&self) -> bool {
    match *self { (_,fin,_) => fin }
  }

  fn fragment_bytes(&self) -> @[u8] {
    match *self { (_,_,bytes) => bytes }
  }
}

fn assert_receiving(reception: Reception) -> Receiver {
  match reception {
    Receiving(r) => { r },
    _ => {
      error!("Receiver wasn't Receiving");
      Unstarted
    }
  }
}
