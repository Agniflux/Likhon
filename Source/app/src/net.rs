//! The only network code in likhon.exe (root CLAUDE.md: no telemetry, no background calls). Every request starts from
//! something the person clicked in Settings (check for updates, the library, the dictionary, donating words), and goes
//! only to the Likhon site (settings::site(), likhon.agniflux.com unless a build or settings.json names another), over https. WinHTTP is Windows'
//! own HTTP client, so there is no extra library and the certificates are the ones Windows trusts. Nothing is sent
//! but the request itself (no cookies, no identifiers).

use windows_sys::Win32::Networking::WinHttp::*;

/// The largest answer read (the dictionary is about 5 MB; fonts and packs are smaller).
const MAX_BYTES: usize = 64 * 1024 * 1024;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The site the app talks to, without a trailing slash.
pub fn site() -> String {
    crate::settings::site()
}

/// Whether `url` is an https address on the Likhon site (the only place the app connects to).
pub fn on_site(url: &str) -> bool {
    let base = site();
    url.starts_with("https://") && (url == base || url.starts_with(&(base.clone() + "/")))
}

/// GET an address on the Likhon site. Returns the body of a 200 answer.
pub fn get(url: &str) -> Result<Vec<u8>, String> {
    request("GET", url, None)
}

/// POST JSON to an address on the Likhon site. Returns the body of a 2xx answer.
pub fn post_json(url: &str, body: &str) -> Result<Vec<u8>, String> {
    request("POST", url, Some(body.as_bytes()))
}

struct Handle(*mut core::ffi::c_void);
impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { WinHttpCloseHandle(self.0) };
        }
    }
}

fn request(method: &str, url: &str, body: Option<&[u8]>) -> Result<Vec<u8>, String> {
    if !on_site(url) {
        return Err("not the Likhon site".into());
    }
    let rest = &url["https://".len()..];
    let (host_port, path) = rest.split_once('/').map(|(h, p)| (h, format!("/{p}"))).unwrap_or((rest, "/".into()));
    let (host, port) = match host_port.split_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().map_err(|_| "bad port")?),
        None => (host_port, 443u16),
    };
    unsafe {
        let session = Handle(WinHttpOpen(wide(&format!("Likhon/{}", env!("CARGO_PKG_VERSION"))).as_ptr(), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, std::ptr::null(), std::ptr::null(), 0));
        if session.0.is_null() {
            return Err("network unavailable".into());
        }
        WinHttpSetTimeouts(session.0, 10_000, 10_000, 15_000, 60_000);
        let connect = Handle(WinHttpConnect(session.0, wide(host).as_ptr(), port, 0));
        if connect.0.is_null() {
            return Err("cannot reach the site".into());
        }
        let req = Handle(WinHttpOpenRequest(connect.0, wide(method).as_ptr(), wide(&path).as_ptr(), std::ptr::null(), std::ptr::null(), std::ptr::null(), WINHTTP_FLAG_SECURE));
        if req.0.is_null() {
            return Err("cannot open the request".into());
        }
        let headers = wide(if body.is_some() { "Content-Type: application/json\r\nAccept: application/json\r\n" } else { "Accept: application/json, */*\r\n" });
        let (ptr, len) = body.map(|b| (b.as_ptr() as *const core::ffi::c_void, b.len() as u32)).unwrap_or((std::ptr::null(), 0));
        if WinHttpSendRequest(req.0, headers.as_ptr(), u32::MAX, ptr, len, len, 0) == 0 || WinHttpReceiveResponse(req.0, std::ptr::null_mut()) == 0 {
            return Err("cannot reach the site".into());
        }
        let mut status: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        WinHttpQueryHeaders(req.0, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, std::ptr::null(), &mut status as *mut u32 as *mut _, &mut size, std::ptr::null_mut());
        let mut out = Vec::new();
        loop {
            let mut avail: u32 = 0;
            if WinHttpQueryDataAvailable(req.0, &mut avail) == 0 {
                return Err("the download stopped".into());
            }
            if avail == 0 {
                break;
            }
            if out.len() + avail as usize > MAX_BYTES {
                return Err("too large".into());
            }
            let start = out.len();
            out.resize(start + avail as usize, 0);
            let mut read: u32 = 0;
            if WinHttpReadData(req.0, out[start..].as_mut_ptr() as *mut _, avail, &mut read) == 0 {
                return Err("the download stopped".into());
            }
            out.truncate(start + read as usize);
        }
        if !(200..300).contains(&status) {
            return Err(format!("the site answered {status}"));
        }
        Ok(out)
    }
}

/// Hex SHA-256 of `bytes` (to check a download against what the site said it is).
pub fn sha256_hex(bytes: &[u8]) -> String {
    use windows_sys::Win32::Security::Cryptography::*;
    unsafe {
        let mut alg = std::ptr::null_mut();
        if BCryptOpenAlgorithmProvider(&mut alg, BCRYPT_SHA256_ALGORITHM, std::ptr::null(), 0) != 0 {
            return String::new();
        }
        let mut hash = [0u8; 32];
        let ok = BCryptHash(alg, std::ptr::null(), 0, bytes.as_ptr() as *mut u8, bytes.len() as u32, hash.as_mut_ptr(), 32) == 0;
        BCryptCloseAlgorithmProvider(alg, 0);
        if !ok {
            return String::new();
        }
        hash.iter().map(|b| format!("{b:02x}")).collect()
    }
}
