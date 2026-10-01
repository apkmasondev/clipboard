use crate::model::Result;
use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::*};

pub fn protect(data: &[u8]) -> Result<Vec<u8>> {
    crypt(data, true)
}
pub fn unprotect(data: &[u8]) -> Result<Vec<u8>> {
    crypt(data, false)
}
fn crypt(data: &[u8], encrypt: bool) -> Result<Vec<u8>> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len().try_into().map_err(|_| "Dane są zbyt duże.")?,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // DPAPI owns the returned allocation; release it using LocalFree after copying.
    unsafe {
        let ok = if encrypt {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err(
                "Windows DPAPI nie może zabezpieczyć lub odczytać danych tego użytkownika.".into(),
            );
        }
        let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData as _);
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_tamper() {
        let text = b"local private clipboard";
        let mut encrypted = protect(text).unwrap();
        assert!(!encrypted.windows(text.len()).any(|w| w == text));
        assert_eq!(unprotect(&encrypted).unwrap(), text);
        let n = encrypted.len();
        encrypted[n - 1] ^= 1;
        assert!(unprotect(&encrypted).is_err());
    }
}
