#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::{
  fs::File,
  io::{BufRead, BufReader, BufWriter, Cursor, Seek, SeekFrom, Write},
  mem,
  str::FromStr,
};

use tiny_http::{Header, Response, Server, StatusCode};
use url::{ParseError, Url};
use uuid::Uuid;

fn main() {
  let mut data_file = File::options()
    .create(true)
    .read(true)
    .write(true)
    .append(true)
    .open("data.txt")
    .unwrap();
  let mut prm_file = File::options()
    .create(true)
    .read(true)
    .write(true)
    .append(true)
    .open("prm.txt")
    .unwrap();

  // (url, token)
  let mut data: Vec<(String, String)> = {
    let mut data = Vec::new();

    // read removed tokens from prm.txt
    let mut prm: Vec<String> = Vec::new();
    let buf = BufReader::new(&prm_file);
    for line in buf.lines() {
      let line = line.unwrap();
      if let Ok(token) = Uuid::from_str(&line) {
        prm.push(token.to_string());
      }
    }

    // read urls from data.txt
    let buf = BufReader::new(&data_file);
    for str in buf.lines().map(|l| l.unwrap()) {
      if str.len() < 36 {
        continue;
      }
      let (token, url) = str.split_at(36);
      data.push((url.to_owned(), token.to_owned()));
    }

    // rewrite data.txt if have pending removals
    if !prm.is_empty() {
      // filter out removed tokens
      data = data
        .into_iter()
        .map(|(url, token)| {
          if prm.iter().any(|t| *token == *t) {
            ("".to_owned(), token)
          } else {
            (url, token)
          }
        })
        .collect();
      // remove file content
      data_file.set_len(0).unwrap();
      data_file.seek(SeekFrom::Start(0)).unwrap();
      // write content to file
      let mut writer = BufWriter::new(&data_file);
      for (url, token) in data.iter() {
        writer
          .write_fmt(format_args!("{}{}\n", token, url))
          .unwrap();
      }
      writer.flush().unwrap();
    }

    // empty prm.txt
    prm_file.set_len(0).unwrap();
    prm_file.seek(SeekFrom::Start(0)).unwrap();

    data
  };

  let server = Server::http("0.0.0.0:8000").unwrap();

  for request in server.incoming_requests() {
    let param = &request.url()[1..];

    let mut param_iter = param.chars();
    let first_char = match param_iter.next() {
      Some(c) => c,
      None => {
        respond_with_error(request, "empty-request", 400);
        continue;
      }
    };

    // add new redirect
    if first_char == '+' {
      // collect rest into a String
      let rest = String::from_iter(param_iter);

      // validate it is a url
      let url = match Url::parse(&rest) {
        Ok(url) => url.to_string(),
        Err(err) => {
          respond_with_error(request, url_parse_error_to_string(err), 400);
          continue;
        }
      };

      // generate a token
      let token = Uuid::new_v4().to_string();

      // write to file
      if writeln!(data_file, "{}{}", token, url).is_err() {
        respond_with_error(request, "write-error", 500);
        continue;
      }

      let json = format!(
        r#"{{"id":"{}","token":"{}"}}"#,
        encode_base36(data.len() as u64),
        token
      );

      // add to data
      data.push((url, token));

      // respond
      let json_len = json.len();
      let res = Response::new(
        StatusCode(200),
        vec![Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()],
        Cursor::new(json.into_bytes()),
        Some(json_len),
        None,
      );
      let _ = request.respond(res);
      continue;
    }

    // remove redirect via token
    if first_char == '-' {
      // collect rest into a String
      let rest = String::from_iter(param_iter);

      // parse token
      let token = match Uuid::parse_str(&rest) {
        Ok(token) => token.to_string(),
        Err(_) => {
          respond_with_error(request, "invalid-token", 400);
          continue;
        }
      };

      // find index
      let index = match data
        .iter()
        .position(|(url, t)| !url.is_empty() && token == *t)
      {
        Some(index) => index,
        None => {
          respond_with_error(request, "not-found", 400);
          continue;
        }
      };

      // pending removal
      if writeln!(prm_file, "{}", token).is_err() {
        respond_with_error(request, "write-error", 500);
        continue;
      }

      // replace url in data with empty string
      let (url, _) = mem::replace(&mut data[index], ("".to_owned(), token));

      // respond
      let res = Response::from_string(url);
      let _ = request.respond(res);
      continue;
    }

    // check existing redirect
    {
      // decode base36 id
      let id = decode_base36(param);
      let id = match id {
        Some(id) => id,
        None => {
          respond_with_error(request, "invalid-id", 400);
          continue;
        }
      };

      // find redirect
      let url = match data.get(id as usize) {
        Some(url) => url,
        None => {
          respond_with_error(request, "not-found", 404);
          continue;
        }
      };

      // if url is empty, respond as not-found
      if url.0.is_empty() {
        respond_with_error(request, "not-found", 404);
        continue;
      }

      // build header
      let location = match Header::from_bytes(&b"Location"[..], url.0.as_bytes()) {
        Ok(header) => header,
        Err(_) => {
          respond_with_error(request, "invalid-url", 400);
          continue;
        }
      };

      // respond
      let res = Response::empty(StatusCode(302)).with_header(location);
      let _ = request.respond(res);
    }
  }
}

fn decode_base36(s: &str) -> Option<u64> {
  let mut result: u64 = 0;
  for c in s.bytes() {
    result = result.checked_mul(36)?;
    if c >= b'0' && c <= b'9' {
      result = result.checked_add((c - b'0') as u64)?;
    } else if c >= b'a' && c <= b'z' {
      result = result.checked_add((c - b'a' + 10) as u64)?;
    } else if c >= b'A' && c <= b'Z' {
      result = result.checked_add((c - b'A' + 10) as u64)?;
    } else {
      return None;
    }
  }
  Some(result)
}

fn encode_base36(n: u64) -> String {
  if n == 0 {
    return "0".to_owned();
  }
  let mut result = String::with_capacity(13);
  let mut n = n;
  while n > 0 {
    let c = (n % 36) as u8;
    if c < 10 {
      result.push((c + b'0') as char);
    } else {
      result.push((c + b'a' - 10) as char);
    }
    n /= 36;
  }
  result.chars().rev().collect()
}

fn respond_with_error(request: tiny_http::Request, error: &str, code: u16) {
  let res = Response::from_string(error).with_status_code(StatusCode(code));
  let _ = request.respond(res);
}

fn url_parse_error_to_string(err: ParseError) -> &'static str {
  match err {
    url::ParseError::EmptyHost => "invalid-url:empty-host",
    url::ParseError::IdnaError => "invalid-url:idna-error",
    url::ParseError::InvalidPort => "invalid-url:invalid-port",
    url::ParseError::InvalidIpv4Address => "invalid-url:invalid-ipv4",
    url::ParseError::InvalidIpv6Address => "invalid-url:invalid-ipv6",
    url::ParseError::InvalidDomainCharacter => "invalid-url:invalid-domain",
    url::ParseError::RelativeUrlWithoutBase => "invalid-url:relative-url-without-base",
    url::ParseError::RelativeUrlWithCannotBeABaseBase => {
      "invalid-url:relative-url-with-cannot-be-a-base-base"
    }
    url::ParseError::SetHostOnCannotBeABaseUrl => "invalid-url:set-host-on-cannot-be-a-base-url",
    url::ParseError::Overflow => "invalid-url:overflow",
    _ => "invalid-url:unknown",
  }
}