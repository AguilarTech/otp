use rand::rngs::OsRng;
use rand::RngCore;
use tempfile::TempDir;
use uuid::Uuid;

use super::error::VaultError;
use super::frame::{HEADER_LEN, MAC_LEN};
use super::Vault;

fn setup_pair(pad_size: usize) -> (Vault, TempDir, Vault, TempDir, Uuid) {
    let alice_dir = TempDir::new().unwrap();
    let bob_dir = TempDir::new().unwrap();
    let alice = Vault::open(alice_dir.path().to_path_buf()).unwrap();
    let bob = Vault::open(bob_dir.path().to_path_buf()).unwrap();

    let id = Uuid::new_v4();
    let mut a_out = vec![0u8; pad_size];
    let mut a_in = vec![0u8; pad_size];
    OsRng.fill_bytes(&mut a_out);
    OsRng.fill_bytes(&mut a_in);

    alice
        .import_pairing(id, "bob".into(), &a_out, &a_in)
        .unwrap();
    // Bob sees the streams with roles swapped: his outbound is Alice's
    // inbound and vice versa.
    bob.import_pairing(id, "alice".into(), &a_in, &a_out)
        .unwrap();

    (alice, alice_dir, bob, bob_dir, id)
}

#[test]
fn roundtrip_short_message() {
    let (alice, _ad, bob, _bd, pid) = setup_pair(1024);
    let plaintext = b"hello, bob";
    let frame = alice.encrypt(&pid, plaintext).unwrap();
    let msg = bob.decrypt(&frame).unwrap();
    assert_eq!(&msg.plaintext[..], plaintext);
    assert_eq!(msg.seq, 1);
    assert_eq!(msg.pairing_id, pid);
}

#[test]
fn roundtrip_two_messages() {
    let (alice, _ad, bob, _bd, pid) = setup_pair(4096);
    let f1 = alice.encrypt(&pid, b"first").unwrap();
    let f2 = alice.encrypt(&pid, b"second message").unwrap();
    let m1 = bob.decrypt(&f1).unwrap();
    let m2 = bob.decrypt(&f2).unwrap();
    assert_eq!(&m1.plaintext[..], b"first");
    assert_eq!(&m2.plaintext[..], b"second message");
    assert_eq!(m1.seq, 1);
    assert_eq!(m2.seq, 2);
}

#[test]
fn mac_fail_on_ciphertext_tamper() {
    let (alice, _ad, bob, _bd, pid) = setup_pair(1024);
    let mut frame = alice.encrypt(&pid, b"secret").unwrap();
    // First byte of ciphertext sits immediately after the header.
    frame[HEADER_LEN] ^= 0x01;
    assert!(matches!(bob.decrypt(&frame), Err(VaultError::MacFailed)));
}

#[test]
fn mac_fail_on_mac_tamper() {
    let (alice, _ad, bob, _bd, pid) = setup_pair(1024);
    let mut frame = alice.encrypt(&pid, b"secret").unwrap();
    let last = frame.len() - 1;
    frame[last] ^= 0x80;
    assert!(matches!(bob.decrypt(&frame), Err(VaultError::MacFailed)));
}

#[test]
fn header_tamper_rejected() {
    let (alice, _ad, bob, _bd, pid) = setup_pair(1024);
    let mut frame = alice.encrypt(&pid, b"secret").unwrap();
    // Flip a bit in the timestamp field (header offset 37).
    frame[37] ^= 0x01;
    assert!(bob.decrypt(&frame).is_err());
    // Even if seq somehow passed, MAC covers the header, so the result is
    // either MacFailed or OffsetOutOfBounds — never a silently-decrypted
    // message.
}

#[test]
fn replay_rejected() {
    let (alice, _ad, bob, _bd, pid) = setup_pair(1024);
    let frame = alice.encrypt(&pid, b"hi").unwrap();
    let _ = bob.decrypt(&frame).unwrap();
    assert!(matches!(
        bob.decrypt(&frame),
        Err(VaultError::Replay { .. })
    ));
}

#[test]
fn out_of_order_delivery_consumes_skipped_pad() {
    let (alice, _ad, bob, _bd, pid) = setup_pair(4096);
    let f1 = alice.encrypt(&pid, b"msg1-XX").unwrap(); // L=7
    let f2 = alice.encrypt(&pid, b"msg2-YY").unwrap(); // L=7
    let m2 = bob.decrypt(&f2).unwrap();
    assert_eq!(&m2.plaintext[..], b"msg2-YY");
    // f1 has seq=1 ≤ last_seq_in=2; rejected as replay.
    assert!(matches!(bob.decrypt(&f1), Err(VaultError::Replay { .. })));
    // Bob's recv_cursor has advanced past where f1's bytes lived.
    let bob_info = bob.list_pairings().unwrap().pop().unwrap();
    assert!(bob_info.in_total - bob_info.in_remaining >= 2 * (32 + 7));
}

#[test]
fn pad_exhausted() {
    // pad_size = 64 → one message of length 32 consumes 32 (mac key) + 32 = 64.
    let (alice, _ad, _b, _bd, pid) = setup_pair(64);
    let big = vec![b'a'; 32];
    let _ = alice.encrypt(&pid, &big).unwrap();
    let result = alice.encrypt(&pid, b"x");
    assert!(matches!(
        result,
        Err(VaultError::PadExhausted {
            needed: 33,
            available: 0
        })
    ));
}

#[test]
fn cursor_and_seq_advance_on_encrypt() {
    let (alice, _ad, _b, _bd, pid) = setup_pair(1024);
    let before = alice.list_pairings().unwrap().pop().unwrap();
    assert_eq!(before.out_remaining, 1024);
    assert_eq!(before.seq_out, 0);

    let _ = alice.encrypt(&pid, b"abc").unwrap();

    let after = alice.list_pairings().unwrap().pop().unwrap();
    assert_eq!(before.out_remaining - after.out_remaining, 32 + 3);
    assert_eq!(after.seq_out, 1);
}

#[test]
fn used_pad_zeroized_on_disk() {
    let (alice, alice_dir, _b, _bd, pid) = setup_pair(256);
    let _ = alice.encrypt(&pid, b"hello").unwrap();
    let pad = std::fs::read(
        alice_dir
            .path()
            .join("pads")
            .join(pid.to_string())
            .join("out.pad"),
    )
    .unwrap();
    let consumed = 32 + 5;
    assert!(
        pad[..consumed].iter().all(|&b| b == 0),
        "first {} bytes should be zero on disk",
        consumed
    );
}

#[test]
fn state_persists_across_reopen() {
    let td = TempDir::new().unwrap();
    let base = td.path().to_path_buf();
    let id;
    {
        let v = Vault::open(base.clone()).unwrap();
        let info = v.create_pairing("persist".into(), 4096).unwrap();
        id = info.id;
        let _ = v.encrypt(&id, b"hi").unwrap();
    }
    let v2 = Vault::open(base).unwrap();
    let listed = v2.list_pairings().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, id);
    assert_eq!(listed[0].seq_out, 1);
    assert_eq!(listed[0].out_remaining, 4096 - 32 - 2);
}

#[test]
fn duplicate_import_rejected() {
    let td = TempDir::new().unwrap();
    let v = Vault::open(td.path().to_path_buf()).unwrap();
    let id = Uuid::new_v4();
    let pad = vec![0xAB; 64];
    v.import_pairing(id, "x".into(), &pad, &pad).unwrap();
    let err = v.import_pairing(id, "y".into(), &pad, &pad).unwrap_err();
    assert!(matches!(err, VaultError::PairingExists(_)));
}

#[test]
fn frame_layout_sanity() {
    let (alice, _ad, _b, _bd, pid) = setup_pair(256);
    let pt = b"abcd";
    let frame = alice.encrypt(&pid, pt).unwrap();
    assert_eq!(frame.len(), HEADER_LEN + pt.len() + MAC_LEN);
}

#[test]
fn empty_plaintext_roundtrip() {
    let (alice, _ad, bob, _bd, pid) = setup_pair(256);
    let frame = alice.encrypt(&pid, b"").unwrap();
    let msg = bob.decrypt(&frame).unwrap();
    assert!(msg.plaintext.is_empty());
    // Even an empty plaintext still consumes 32 bytes (mac key).
    let info = alice.list_pairings().unwrap().pop().unwrap();
    assert_eq!(info.out_total - info.out_remaining, 32);
}
