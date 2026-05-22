use poly1305::universal_hash::{KeyInit, UniversalHash};
use poly1305::Poly1305;
use subtle::ConstantTimeEq;

use super::error::{Result, VaultError};
use super::frame::MAC_LEN;

pub fn compute_mac(key: &[u8; 32], data: &[u8]) -> [u8; MAC_LEN] {
    let k = poly1305::Key::from_slice(key);
    let mut mac = Poly1305::new(k);
    mac.update_padded(data);
    let tag = mac.finalize();
    let mut out = [0u8; MAC_LEN];
    out.copy_from_slice(&tag);
    out
}

pub fn verify_mac(key: &[u8; 32], data: &[u8], expected: &[u8; MAC_LEN]) -> Result<()> {
    let computed = compute_mac(key, data);
    if computed.ct_eq(expected).into() {
        Ok(())
    } else {
        Err(VaultError::MacFailed)
    }
}

pub fn xor_in_place(buf: &mut [u8], keystream: &[u8]) {
    debug_assert_eq!(buf.len(), keystream.len());
    for (b, k) in buf.iter_mut().zip(keystream.iter()) {
        *b ^= *k;
    }
}
