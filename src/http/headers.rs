use std::treemap::*;

pub trait Headers {
  fn get_header(&self, name: &str) -> Option<~str>;
  fn has_header(&self, name: &str) -> bool;
}

pub trait HeadersUtil {
  fn has_header_value(&self, name: &str, value: &str) -> bool;
  fn has_header_keyword(&self, name: &str, value: &str) -> bool;
}

pub trait HeadersMutable {
  fn set_header(&mut self, name: &str, value: &str);
  fn remove_header(&mut self, name: &str);
}

struct HeaderMap(TreeMap<~str,~str>);

impl HeaderMap {
  pub fn new() -> HeaderMap { HeaderMap(TreeMap::new()) }
}

impl Headers for HeaderMap {
  fn has_header(&self, name: &str) -> bool {
    let our_name = str::from_slice(name).to_lower();
    self.contains_key(&our_name)
  }

  fn get_header(&self, name: &str) -> Option<~str> {
    let our_name = str::from_slice(name).to_lower();
    let found = self.find(&our_name);
    found.map(|s| s.clone())
  }
}

impl HeadersMutable for HeaderMap {
  fn set_header(&mut self, name: &str, value: &str) {
    let our_name = str::from_slice(name).to_lower();
    let our_value = str::from_slice(value);
    self.insert(our_name, our_value);
  }

  fn remove_header(&mut self, name: &str) {
    let our_name = str::from_slice(name).to_lower();
    self.remove(&our_name);
  }
}

impl<T: Headers> HeadersUtil for T {
  fn has_header_value(&self, name: &str, value: &str) -> bool{
    let actual_value = self.get_header(name);
    actual_value == Some(str::from_slice(value))
  }

  fn has_header_keyword(&self, name: &str, value: &str) -> bool{
    let actual_value = self.get_header(name);

    match actual_value {
      Some(actual_str) => contains_keyword(actual_str, value),
      _ => false
    }
  }
}

fn contains_keyword(haystack: &str, needle: &str) -> bool {
  let lower_needle = needle.to_lower();

  for haystack.each_split_char(',') |word| {
    if lower_needle == word.trim().to_lower() {
      return true
    }
  }

  false
}


#[test]
fn get_header_test() {
  let mut headers = HeaderMap::new();

  headers.set_header("foO", "paNts");

  assert!(headers.get_header("foo") == Some(~"paNts"));
  assert!(headers.get_header("Foo") == Some(~"paNts"));
}

#[test]
fn has_header_test() {
  let mut headers = HeaderMap::new();

  headers.set_header("foO", "paNts");

  assert!(headers.has_header("foo"));
  assert!(headers.has_header("Foo"));
}

#[test]
fn has_header_value_test() {
  let mut headers = HeaderMap::new();

  headers.set_header("foO", "paNts");

  assert!(headers.has_header_value("foo","paNts"));
  assert!(!headers.has_header_value("foo","pants"));
}

#[test]
fn has_header_keyword_test() {
  let mut headers = HeaderMap::new();

  headers.set_header("foO", "shoes, paNts, frocks");

  assert!(headers.has_header_keyword("foo","paNts"));
  assert!(headers.has_header_keyword("foo","pants"));
}

#[test]
fn remove_header_test() {
  let mut headers = HeaderMap::new();

  headers.set_header("foO", "paNts");
  headers.remove_header("fOo");

  assert!(!headers.has_header("foo"));
}
