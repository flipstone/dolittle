#[deriving(Eq)]
pub enum Method {
  DELETE, GET, HEAD, POST, PUT, CONNECT, OPTIONS,
  TRACE, COPY, LOCK, MKCOL, MOVE, PROPFIND, PROPPATCH,
  SEARCH, UNLOCK, REPORT, MKACTIVITY, CHECKOUT, MERGE,
  MSEARCH, NOTIFY, SUBSCRIBE, UNSUBSCRIBE, PATCH, PURGE
}

#[deriving(Eq)]
pub struct HttpVersion(u16,u16);

pub trait Request {
  fn http_version(&self) -> Option<HttpVersion>;
  fn method(&self) -> Option<Method>;
}
