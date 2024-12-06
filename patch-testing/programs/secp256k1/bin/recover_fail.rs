#![no_main]
sp1_zkvm::entrypoint!(main);

use secp256k1::{
    Secp256k1,
    Message as Secp256k1Message,
};

pub fn main() {
    let secp = Secp256k1::new();
    let key = secp256k1::Keypair::new(&secp, &mut secp256k1::rand::thread_rng());

    let message = Secp256k1Message::from_digest_slice(&[0; 32]).unwrap();

    let sig = secp.sign_ecdsa_recoverable(&message, &key.secret_key());

    let recovered_result = secp.recover_ecdsa(&Secp256k1Message::from_digest_slice(&[1; 32]).unwrap(), &sig).unwrap();

    assert_ne!(recovered_result, key.public_key());
}
