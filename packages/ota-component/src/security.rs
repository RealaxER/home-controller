use openssl::pkey::PKey;
use std::fs::File;
use std::io::{Read, Write};
use openssl::hash::MessageDigest;
use openssl::pkey::Public;
use std::fs::OpenOptions;
use std::io::Seek;
use openssl::sign::{Signer, Verifier};

use crate::error::OtaErr;

pub struct DsaType {
    f_path: String,
    public_key: PKey<Public>
}

impl DsaType {
    pub fn new(f_path: String, kpublic_path: String) -> Self {

        let mut public_key_pem = Vec::new();
        File::open(kpublic_path.clone())
            .unwrap()
            .read_to_end(&mut public_key_pem)
            .unwrap();

        let public_key = PKey::public_key_from_pem(&public_key_pem).unwrap();

        DsaType {
            f_path: f_path,
            public_key: public_key
        }
    }
    pub async fn verify(&mut self,len:usize) -> Result<(), OtaErr> {        
        let mut file_content = Vec::new();
        let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&self.f_path)
        .expect("Failed to open data file");

        file.read_to_end(&mut file_content)
        .expect("Failed to read data file");

        let file_len = file_content.len();

        let mut temp = &file_content[0..(file_len - len)];

        let last_256_bytes = &file_content[file_len.saturating_sub(len)..];

        file.seek(std::io::SeekFrom::Start(0))
        .expect("Failed to seek back to the beginning of the file");

        let _ =  file.write_all(&mut temp);

        // verify
        let mut verifier = Verifier::new(MessageDigest::sha256(), &self.public_key)
        .expect("Failed to create verifier");

        match verifier.verify_oneshot(&last_256_bytes, &temp) {
            Ok(check) => {
                if check {
                    return Ok(());
                } else {
                    return Err(OtaErr::VerifyNotEqualErr);
                }
            }
            Err(err) => {
                println!("Error verifying: {}", err);
                return Err(OtaErr::VerifyErr);
            }
        }
    }

    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, OtaErr> {
        let mut private_key_pem = Vec::new();
        File::open("private_key.pem") // Update the path to your private key
            .expect("Failed to open private key file")
            .read_to_end(&mut private_key_pem)
            .expect("Failed to read private key file");

        let private_key = PKey::private_key_from_pem(&private_key_pem)
            .expect("Failed to create private key from PEM");

        let mut signer = Signer::new(MessageDigest::sha256(), &private_key)
            .expect("Failed to create signer");

        signer.update(&message).expect("Failed to update signer with message");
        let signature = signer.sign_to_vec().expect("Failed to sign message");

        Ok(signature)
    }
}
