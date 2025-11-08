#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub protocol: String,
    pub headers: Vec<String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: String,
    pub protocol: String,
    pub headers: Vec<String>,
    pub content_type: String,
    pub body: Vec<u8>,
    pub end: bool,
}

impl HttpRequest {
    // Associated function (constructor)
    fn new() -> Self {
        Self {method: String::from(""), path: String::from(""), protocol: String::from(""), headers: Vec::new(), body: Vec::new()}
    }
}

pub fn generate_httprequest(data: &mut Vec<u8>) -> HttpRequest {
    let mut headers: &[u8] = &[];
    let mut body: &[u8] = &[];
    if let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n") {
        // pos = index where \r\n\r\n starts
        headers = &data[..pos];       // all bytes up to (but not including) the marker
        body = &data[pos + 4..];      // everything after \r\n\r\n
    }
    let mut request_data = HttpRequest::new();
    request_data.body.extend_from_slice(body);
    let headers1 = String::from_utf8_lossy(headers).into_owned();
    request_data.headers = String::from_utf8_lossy(headers)
    .lines()
    .map(String::from)
    .collect();
    let request_headers_split: Vec<&str> = headers1.split("\r\n").collect();
    let mut word_index: i32 = 0;
    for word in request_headers_split[0].split(" ") {
        match word_index {
            0 => request_data.method = String::from(word),
            1 => request_data.path = String::from(word),
            2 => request_data.protocol = String::from(word),
            _ => log::warn!("[PLEASE FIX ME!!!] While parsing the top HTTP request line, an abnormal index of {} was found, it's value is: {}", word_index, word),
        }
        word_index = word_index+1;
    }
    return request_data;
}