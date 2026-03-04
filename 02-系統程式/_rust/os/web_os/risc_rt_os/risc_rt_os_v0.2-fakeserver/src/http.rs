// 簡單的 HTTP 響應處理
use core::fmt::Write;

pub struct HttpResponse {
    buffer: [u8; 1024],
    len: usize,
}

impl HttpResponse {
    pub fn new() -> Self {
        HttpResponse {
            buffer: [0; 1024],
            len: 0,
        }
    }

    pub fn ok(mut self, body: &str) -> Self {
        self.len = 0;
        let _ = write!(
            &mut self,
            "HTTP/1.1 200 OK\r\n\
             Content-Type: text/html; charset=utf-8\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            body.len(),
            body
        );
        self
    }

    pub fn not_found(mut self) -> Self {
        self.len = 0;
        let body = "<html><body><h1>404 Not Found</h1></body></html>";
        let _ = write!(
            &mut self,
            "HTTP/1.1 404 Not Found\r\n\
             Content-Type: text/html; charset=utf-8\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            body.len(),
            body
        );
        self
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.len]
    }
}

impl Write for HttpResponse {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.buffer.len() - self.len;
        if bytes.len() > remaining {
            return Err(core::fmt::Error);
        }
        self.buffer[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(())
    }
}

pub fn parse_request(data: &[u8]) -> Option<&str> {
    let request = core::str::from_utf8(data).ok()?;
    let first_line = request.lines().next()?;
    
    // 手動解析，不使用 Vec
    let mut parts = first_line.split_whitespace();
    let method = parts.next()?;
    let path = parts.next()?;
    
    if method == "GET" {
        Some(path)
    } else {
        None
    }
}