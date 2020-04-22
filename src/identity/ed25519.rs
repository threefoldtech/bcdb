use ed25519_dalek::{
    Keypair, PublicKey as PubKey, SecretKey, Signature as Sig, KEYPAIR_LENGTH, SECRET_KEY_LENGTH,
    SIGNATURE_LENGTH,
};
use serde::de::{Error as SerdeError, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::fmt;

use super::Error;

/// An identity representation on the threefold grid. This can be used to sign messages, and verify
/// signatures of messages created by this identity. The public key can be exported and exchanged
/// to allow others to verify messages signed with this identity.
#[derive(Debug)]
pub struct Identity {
    kp: Keypair,
}

/// A stand alone public key, which can be used to verify signatures made with the associated
/// private key.
#[derive(Debug)]
pub struct PublicKey {
    pk: PubKey,
}

/// A detached signature. The message itself is not stored.
#[derive(Debug)]
pub struct Signature(Sig);

impl Identity {
    pub fn from_sk_bytes(sk_bytes: &[u8]) -> Result<Identity, Error> {
        // The used lib only allows us to use an existing keypair by loading it from bytes, but we
        // also need the public key bytes, which we usually don't have, so load the private key,
        // use that to generate the public key, copy them to a new array, and use that to load an
        // identity
        let sk = SecretKey::from_bytes(sk_bytes).map_err(|_| Error::MalformedPrivateKey)?;
        let pk: PubKey = (&sk).into();
        let mut kp_bytes = [0; KEYPAIR_LENGTH];
        kp_bytes[..SECRET_KEY_LENGTH].clone_from_slice(sk_bytes);
        kp_bytes[SECRET_KEY_LENGTH..].clone_from_slice(pk.as_bytes());
        // we already verified the private key above, and generated the correct public key using
        // the lib, so this can't fail unless the lib is faulty in itself
        let kp = Keypair::from_bytes(&kp_bytes).unwrap();
        Ok(Identity { kp })
    }

    /// Create a detached signature for a message.
    pub fn sign(&self, msg: &[u8]) -> Signature {
        Signature(self.kp.sign(msg))
    }

    /// Verify a detached signature for a message.
    pub fn verify(&self, msg: &[u8], sig: &Signature) -> Result<(), Error> {
        self.kp
            .verify(msg, &sig.0)
            .map_err(|_| Error::InvalidSignature)
    }
}

impl PublicKey {
    pub fn from_bytes(pk_bytes: &[u8]) -> Result<PublicKey, Error> {
        let pk = PubKey::from_bytes(pk_bytes).map_err(|_| Error::MalformedPublicKey)?;
        Ok(PublicKey { pk })
    }

    pub fn verify(&self, msg: &[u8], sig: &Signature) -> Result<(), Error> {
        self.pk
            .verify(msg, &sig.0)
            .map_err(|_| Error::InvalidSignature)
    }
}

impl Signature {
    pub fn from_bytes(sig_bytes: &[u8]) -> Result<Signature, Error> {
        Ok(Signature(
            Sig::from_bytes(sig_bytes).map_err(|_| Error::MalformedSignature)?,
        ))
    }

    /// Convert the signature to raw bytes
    pub fn to_bytes(&self) -> [u8; SIGNATURE_LENGTH] {
        self.0.to_bytes()
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(&self.pk.as_bytes()))
        } else {
            serializer.serialize_bytes(self.pk.as_bytes())
        }
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<PublicKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_string(PublicKeyVisitor)
        } else {
            deserializer.deserialize_bytes(PublicKeyVisitor)
        }
    }
}

struct PublicKeyVisitor;

impl<'de> Visitor<'de> for PublicKeyVisitor {
    type Value = PublicKey;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an encoded public key")
    }

    fn visit_str<E>(self, hex_str: &str) -> Result<PublicKey, E>
    where
        E: SerdeError,
    {
        let bytes = hex::decode(hex_str).or_else(|_| {
            Err(SerdeError::invalid_value(
                Unexpected::Str(hex_str),
                &"a hex string",
            ))
        })?;
        Ok(PublicKey {
            pk: PubKey::from_bytes(&bytes)
                .or_else(|_| Err(SerdeError::invalid_length(bytes.len(), &self)))?,
        })
    }

    fn visit_bytes<E>(self, bytes: &[u8]) -> Result<PublicKey, E>
    where
        E: SerdeError,
    {
        Ok(PublicKey {
            pk: PubKey::from_bytes(bytes)
                .or_else(|_| Err(SerdeError::invalid_length(bytes.len(), &self)))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;

    #[test]
    fn load_private_key() {
        let key = vec![0; 8];
        Identity::from_sk_bytes(&key).unwrap_err();
        let key = vec![0; 16];
        Identity::from_sk_bytes(&key).unwrap_err();
        let key = vec![0; 24];
        Identity::from_sk_bytes(&key).unwrap_err();
        let key = vec![0; 31];
        Identity::from_sk_bytes(&key).unwrap_err();
        let key = vec![0; 32];
        Identity::from_sk_bytes(&key).unwrap();
        let key = vec![0; 33];
        Identity::from_sk_bytes(&key).unwrap_err();
        let key = vec![0; 48];
        Identity::from_sk_bytes(&key).unwrap_err();
        let key = vec![0; 64];
        Identity::from_sk_bytes(&key).unwrap_err();
    }

    #[test]
    fn sign_message() {
        let sk = vec![0; 32];
        let id = Identity::from_sk_bytes(&sk).unwrap();

        let message = b"the message to sign";
        let sig = id.sign(message);
        id.verify(message, &sig).unwrap();
    }

    #[test]
    fn decode_public_key_json() {
        let invalid_type = "35498";
        serde_json::from_str::<PublicKey>(&invalid_type).unwrap_err();
        let invalid_len_hex = "\"32daf139faden\"";
        serde_json::from_str::<PublicKey>(&invalid_len_hex).unwrap_err();
        // unlike private keys, not all byte slices represent a public key
        let invalid_point_hex =
            "\"32daf139fade12343509784353eb4a1cd85e91ff4f0900af3fc4b8907aade3cc\"";
        serde_json::from_str::<PublicKey>(&invalid_point_hex).unwrap_err();
        let valid_hex = "\"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29\"";
        serde_json::from_str::<PublicKey>(&valid_hex).unwrap();
    }

    #[test]
    fn encode_public_key_json() {
        let pk_bytes = &[
            59, 106, 39, 188, 206, 182, 164, 45, 98, 163, 168, 208, 42, 111, 13, 115, 101, 50, 21,
            119, 29, 226, 67, 166, 58, 192, 72, 161, 139, 89, 218, 41,
        ];

        let pk = PublicKey::from_bytes(pk_bytes).unwrap();
        let json_key = serde_json::to_string(&pk).unwrap();

        assert_eq!(
            "\"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29\"",
            &json_key
        );
    }
}