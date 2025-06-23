use rand::rngs::OsRng;
use rand::RngCore;

fn generate_random_bytes(size: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; size];
    let mut rng = OsRng;
    rng.fill_bytes(&mut bytes);
    bytes
}

fn generate_random_nonce() -> Vec<u8> {
    generate_random_bytes(12)
}

fn generate_rsa_key_pair() -> (Vec<u8>, Vec<u8>) {
    let secret_key = generate_random_bytes(256);
    let public_key = generate_random_bytes(256);
    (secret_key, public_key)
}

fn generate_aes_key() -> Vec<u8> {
    generate_random_bytes(32)
}


