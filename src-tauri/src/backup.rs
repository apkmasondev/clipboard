use crate::model::Result;
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand_core::{OsRng, RngCore};
use std::{
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"SCBACK01";
pub const MAX_BACKUP: usize = 80 * 1024 * 1024;
const HEADER: usize = 8 + 16 + 12;

fn key(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>> {
    if password.chars().count() < 12 || password.len() > 1024 {
        return Err("Hasło kopii musi mieć co najmniej 12 znaków i maksymalnie 1024 bajty.".into());
    }
    let params = Params::new(65536, 3, 1, Some(32)).map_err(|_| "Błąd parametrów szyfrowania.")?;
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, key.as_mut())
        .map_err(|_| "Nie można przygotować klucza kopii.")?;
    Ok(key)
}
pub fn encrypt(data: &[u8], password: &str) -> Result<Vec<u8>> {
    if data.len() > MAX_BACKUP - HEADER - 16 {
        return Err("Kopia przekracza 80 MB.".into());
    }
    let mut header = vec![0u8; HEADER];
    header[..8].copy_from_slice(MAGIC);
    OsRng
        .try_fill_bytes(&mut header[8..])
        .map_err(|_| "Brak bezpiecznej losowości Windows.")?;
    let key = key(password, &header[8..24])?;
    let cipher = Aes256Gcm::new_from_slice(key.as_ref()).map_err(|_| "Błąd klucza kopii.")?;
    let encrypted = cipher
        .encrypt(
            Nonce::from_slice(&header[24..]),
            Payload {
                msg: data,
                aad: &header,
            },
        )
        .map_err(|_| "Nie można zaszyfrować kopii.")?;
    header.extend_from_slice(&encrypted);
    Ok(header)
}
pub fn decrypt(data: &[u8], password: &str) -> Result<Zeroizing<Vec<u8>>> {
    if data.len() < HEADER + 16 || data.len() > MAX_BACKUP || &data[..8] != MAGIC {
        return Err("Nieprawidłowy format lub rozmiar kopii Super Clipboard.".into());
    }
    let key = key(password, &data[8..24])?;
    let cipher = Aes256Gcm::new_from_slice(key.as_ref()).map_err(|_| "Błąd klucza kopii.")?;
    cipher
        .decrypt(
            Nonce::from_slice(&data[24..HEADER]),
            Payload {
                msg: &data[HEADER..],
                aad: &data[..HEADER],
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| "Nieprawidłowe hasło lub uszkodzona kopia. Niczego nie zmieniono.".into())
}
pub fn read_bounded(path: &Path, max: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| "Nie można otworzyć pliku.")?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Nie można odczytać pliku.")?;
    if bytes.len() > max {
        return Err("Plik przekracza dozwolony rozmiar.".into());
    }
    Ok(bytes)
}
pub fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or("Nieprawidłowy katalog docelowy.")?;
    let mut file = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| "Nie można utworzyć pliku tymczasowego.")?;
    file.write_all(data)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|_| "Nie udało się zapisać całego pliku. Poprzednia kopia pozostaje bez zmian.")?;
    file.persist(path)
        .map_err(|_| "Nie można zastąpić pliku. Sprawdź uprawnienia i wolne miejsce.")?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_authenticated_backup() {
        let password = "test hasło dla kopii 🦀";
        let encrypted = encrypt(b"private snippet", password).unwrap();
        assert_eq!(
            &**decrypt(&encrypted, password).unwrap(),
            b"private snippet"
        );
        assert_ne!(encrypted, encrypt(b"private snippet", password).unwrap());
        assert!(decrypt(&encrypted, "inne haslo do testu").is_err());
        let mut changed = encrypted.clone();
        changed[HEADER] ^= 1;
        assert!(decrypt(&changed, password).is_err());
        assert!(decrypt(&encrypted[..HEADER], password).is_err());
        assert!(encrypt(b"x", "short").is_err());
    }
    #[test]
    fn bounded_read_and_atomic_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("backup.scbackup");
        write_atomic(&path, b"old").unwrap();
        write_atomic(&path, b"new data").unwrap();
        assert_eq!(read_bounded(&path, 8).unwrap(), b"new data");
        assert!(read_bounded(&path, 7).is_err());
    }
}
