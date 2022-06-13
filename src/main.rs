#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::{
  fs::{self, File},
  io::{BufRead, BufReader, BufWriter, Cursor, Write},
  mem,
};

use tiny_http::{Header, Method, Response, Server, StatusCode};
use url::{ParseError, Url};
use uuid::Uuid;

fn main() {
  // (url, token)
  let mut data: Vec<(String, String)> = {
    let mut data = Vec::new();

    // open data files
    let data_file = File::options()
      .create(true)
      .read(true)
      .write(true)
      .open("data.txt")
      .unwrap();
    let prm_file = File::options()
      .create(true)
      .read(true)
      .write(true)
      .open("prm.txt")
      .unwrap();

    // read removed tokens from `prm.txt`
    let mut prm: Vec<u64> = Vec::new();
    let buf = BufReader::new(&prm_file);
    for line in buf.lines() {
      let line = line.unwrap();
      if let Some(id) = decode_base36(&line) {
        prm.push(id);
      }
    }

    // read urls from `data.txt`
    let buf = BufReader::new(&data_file);
    for str in buf.lines().map(|l| l.unwrap()) {
      if str.len() < 36 {
        continue;
      }
      let (token, url) = str.split_at(36);
      data.push((url.to_owned(), token.to_owned()));
    }

    // close files
    drop(data_file);
    drop(prm_file);

    // rewrite `data.txt` if have pending removals
    if !prm.is_empty() {
      // replace removed redirects with empty url
      for i in prm {
        data[i as usize].0 = "".to_owned();
      }

      // create a new data file
      let data2_file = File::options()
        .create_new(true)
        .write(true)
        .append(true)
        .open("data2.txt")
        .unwrap();

      // write content to file
      let mut writer = BufWriter::new(&data2_file);
      for (url, token) in data.iter() {
        writer
          .write_fmt(format_args!("{}{}\n", token, url))
          .unwrap();
      }
      writer.flush().unwrap();
      drop(writer);
      drop(data2_file);

      // replace `data.txt` with `data2.txt`
      fs::rename("data2.txt", "data.txt").unwrap();

      // remove `prm.txt`
      fs::remove_file("prm.txt").unwrap();
    }

    data
  };

  let mut data_file = File::options()
    .create(true)
    .write(true)
    .append(true)
    .open("data.txt")
    .unwrap();
  let mut prm_file = File::options()
    .create(true)
    .write(true)
    .append(true)
    .open("prm.txt")
    .unwrap();

  let server = Server::http("0.0.0.0:8000").unwrap();
  println!("Server listening on http://localhost:8000/");

  for request in server.incoming_requests() {
    let method = request.method();
    let param = {
      let mut chars = request.url().chars();
      chars.next();
      chars.as_str()
    };

    // add new redirect
    if Method::Post == *method {
      let url = param.to_owned();

      // validate it is a url
      let url = match Url::parse(&url) {
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
    if Method::Delete == *method {
      let rest = param.to_owned();

      // parse id && token from input
      let mut elems = rest.splitn(2, '-');
      let id_str = match elems.next() {
        Some(id_str) => id_str,
        None => {
          respond_with_error(request, "invalid-id", 400);
          continue;
        }
      };
      let id = match decode_base36(id_str) {
        Some(id) => id,
        None => {
          respond_with_error(request, "invalid-id", 400);
          continue;
        }
      };
      let token = match elems.next().and_then(|token| Uuid::parse_str(token).ok()) {
        Some(token) => token.to_string(),
        None => {
          respond_with_error(request, "invalid-token", 400);
          continue;
        }
      };

      // find index
      let item = match data.get(id as usize) {
        Some(item) => item,
        None => {
          respond_with_error(request, "not-found", 400);
          continue;
        }
      };

      // if the token is incorrect, respond as invalid token
      if item.1 != token {
        respond_with_error(request, "invalid-token", 400);
        continue;
      }

      // if url is empty, respond as not-found
      if item.0.is_empty() {
        respond_with_error(request, "not-found", 400);
        continue;
      }

      // pending removal
      if writeln!(prm_file, "{}", id_str).is_err() {
        respond_with_error(request, "write-error", 500);
        continue;
      }

      // replace url in data with empty string
      let (url, _) = mem::replace(&mut data[id as usize], ("".to_owned(), token));

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
      let item = match data.get(id as usize) {
        Some(url) => url,
        None => {
          respond_with_error(request, "not-found", 404);
          continue;
        }
      };

      // if url is empty, respond as not-found
      if item.0.is_empty() {
        respond_with_error(request, "not-found", 404);
        continue;
      }

      // build header
      let location = match Header::from_bytes(b"Location".as_slice(), item.0.as_bytes()) {
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
    if (b'0'..=b'9').contains(&c) {
      result = result.checked_add((c - b'0') as u64)?;
    } else if (b'a'..=b'z').contains(&c) {
      result = result.checked_add((c - b'a' + 10) as u64)?;
    } else if (b'A'..=b'Z').contains(&c) {
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
