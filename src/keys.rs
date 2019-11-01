// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// https://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

//! Module providing keys, keypairs, and signatures.
//!
//! The easiest way to get a `PublicKey` is to create a random `Keypair` first through one of the
//! `new` functions. A `PublicKey` can't be generated by itself; it must always be derived from a
//! secret key.

use crate::{utils, Ed25519Digest, Error, Result, XorName, XOR_NAME_LEN};
use ed25519_dalek;
use hex_fmt::HexFmt;
use multibase::Decodable;
use rand::{CryptoRng, Rng};
use serde::{Deserialize, Serialize, Serializer};
use std::{
    cmp::Ordering,
    fmt::{self, Debug, Display, Formatter},
    hash::{Hash, Hasher},
};
use threshold_crypto::{self, serde_impl::SerdeSecret};
use unwrap::unwrap;

/// Wrapper for different public key types.
#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum PublicKey {
    /// Ed25519 public key.
    Ed25519(ed25519_dalek::PublicKey),
    /// BLS public key.
    Bls(threshold_crypto::PublicKey),
    /// BLS public key share.
    BlsShare(threshold_crypto::PublicKeyShare),
}

impl PublicKey {
    /// Returns the BLS key, if applicable.
    pub fn bls(&self) -> Option<threshold_crypto::PublicKey> {
        if let Self::Bls(bls) = self {
            Some(*bls)
        } else {
            None
        }
    }

    /// Returns `Ok(())` if `signature` matches the message and `Err(Error::InvalidSignature)`
    /// otherwise.
    pub fn verify<T: AsRef<[u8]>>(&self, signature: &Signature, data: T) -> Result<()> {
        let is_valid = match (self, signature) {
            (Self::Ed25519(pub_key), Signature::Ed25519(sig)) => {
                pub_key.verify::<Ed25519Digest>(data.as_ref(), sig).is_ok()
            }
            (Self::Bls(pub_key), Signature::Bls(sig)) => pub_key.verify(sig, data),
            (Self::BlsShare(pub_key), Signature::BlsShare(sig)) => pub_key.verify(sig, data),
            _ => return Err(Error::SigningKeyTypeMismatch),
        };
        if is_valid {
            Ok(())
        } else {
            Err(Error::InvalidSignature)
        }
    }

    /// Returns the `PublicKey` serialised and encoded in z-base-32.
    pub fn encode_to_zbase32(&self) -> String {
        utils::encode(&self)
    }

    /// Creates from z-base-32 encoded string.
    pub fn decode_from_zbase32<I: Decodable>(encoded: I) -> Result<Self> {
        utils::decode(encoded)
    }
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for PublicKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        utils::serialise(&self).hash(state)
    }
}

impl Ord for PublicKey {
    fn cmp(&self, other: &PublicKey) -> Ordering {
        utils::serialise(&self).cmp(&utils::serialise(other))
    }
}

impl PartialOrd for PublicKey {
    fn partial_cmp(&self, other: &PublicKey) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<&SecretKey> for PublicKey {
    fn from(sec_key: &SecretKey) -> PublicKey {
        sec_key.public_key()
    }
}

impl From<PublicKey> for XorName {
    fn from(public_key: PublicKey) -> Self {
        let bytes = match public_key {
            PublicKey::Ed25519(pub_key) => {
                return XorName(pub_key.to_bytes());
            }
            PublicKey::Bls(pub_key) => pub_key.to_bytes(),
            PublicKey::BlsShare(pub_key) => pub_key.to_bytes(),
        };
        let mut xor_name = XorName::default();
        xor_name.0.clone_from_slice(&bytes[..XOR_NAME_LEN]);
        xor_name
    }
}

impl From<ed25519_dalek::PublicKey> for PublicKey {
    fn from(public_key: ed25519_dalek::PublicKey) -> Self {
        Self::Ed25519(public_key)
    }
}

impl From<threshold_crypto::PublicKey> for PublicKey {
    fn from(public_key: threshold_crypto::PublicKey) -> Self {
        Self::Bls(public_key)
    }
}

impl From<threshold_crypto::PublicKeyShare> for PublicKey {
    fn from(public_key: threshold_crypto::PublicKeyShare) -> Self {
        Self::BlsShare(public_key)
    }
}

impl Debug for PublicKey {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "PublicKey::")?;
        match self {
            Self::Ed25519(pub_key) => {
                write!(formatter, "Ed25519({:<8})", HexFmt(&pub_key.to_bytes()))
            }
            Self::Bls(pub_key) => write!(
                formatter,
                "Bls({:<8})",
                HexFmt(&pub_key.to_bytes()[..XOR_NAME_LEN])
            ),
            Self::BlsShare(pub_key) => write!(
                formatter,
                "BlsShare({:<8})",
                HexFmt(&pub_key.to_bytes()[..XOR_NAME_LEN])
            ),
        }
    }
}

impl Display for PublicKey {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(self, formatter)
    }
}

// TODO: Remove SecretKey?

/// Wrapper type for different secret key types.
#[derive(Deserialize)]
pub enum SecretKey {
    /// Ed25519 secret key.
    Ed25519(ed25519_dalek::SecretKey),
    /// BLS secret key.
    Bls(threshold_crypto::SecretKey),
    /// BLS secret key share.
    BlsShare(threshold_crypto::SecretKeyShare),
}

// Need to manually implement this due to a missing impl in `Ed25519::SecretKey`.
impl Clone for SecretKey {
    fn clone(&self) -> Self {
        match self {
            Self::Ed25519(sec_key) => Self::Ed25519(unwrap!(ed25519_dalek::SecretKey::from_bytes(
                &sec_key.to_bytes()
            ))),
            Self::Bls(sec_key) => Self::Bls(sec_key.clone()),
            Self::BlsShare(sec_key) => Self::BlsShare(sec_key.clone()),
        }
    }
}

// Need to manually implement this due to a missing impl in `Ed25519::SecretKey`.
impl PartialEq for SecretKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ed25519(sec_key), Self::Ed25519(other_sec_key)) => {
                sec_key.to_bytes() == other_sec_key.to_bytes()
            }
            (Self::Bls(sec_key), Self::Bls(other_sec_key)) => sec_key == other_sec_key,
            (Self::BlsShare(sec_key), Self::BlsShare(other_sec_key)) => sec_key == other_sec_key,
            _ => false,
        }
    }
}

// Need to manually implement this due to a missing impl in `Ed25519::SecretKey`.
impl Eq for SecretKey {}

// Need to manually implement this due to a missing impl in `threshold_crypto::SecretKey`.
impl Serialize for SecretKey {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Ed25519(sec_key) => {
                serializer.serialize_newtype_variant("SecretKey", 0, "Ed25519", &sec_key)
            }
            Self::Bls(sec_key) => {
                serializer.serialize_newtype_variant("SecretKey", 1, "Bls", &SerdeSecret(&sec_key))
            }
            Self::BlsShare(sec_key) => serializer.serialize_newtype_variant(
                "SecretKey",
                2,
                "BlsShare",
                &SerdeSecret(&sec_key),
            ),
        }
    }
}

impl SecretKey {
    /// Generates a random BLS key.
    pub fn new_bls<T: CryptoRng + Rng>(rng: &mut T) -> Self {
        Self::Bls(rng.gen::<threshold_crypto::SecretKey>())
    }

    // TODO: constructors for the other variants.

    /// Returns the corresponding public key.
    ///
    /// Equivalent to calling `PublicKey::from(&self)`.
    pub fn public_key(&self) -> PublicKey {
        match self {
            Self::Ed25519(ref sec_key) => {
                // TODO: Use `PublicKey::from` instead once ed25519 v0.10.0 is released.
                PublicKey::Ed25519(ed25519_dalek::PublicKey::from_secret::<Ed25519Digest>(
                    sec_key,
                ))
            }
            Self::Bls(sec_key) => PublicKey::Bls(sec_key.public_key()),
            Self::BlsShare(sec_key) => PublicKey::BlsShare(sec_key.public_key_share()),
        }
    }

    // TODO: implement zbase32 encoding and decoding
}

impl From<ed25519_dalek::SecretKey> for SecretKey {
    fn from(secret_key: ed25519_dalek::SecretKey) -> Self {
        Self::Ed25519(secret_key)
    }
}

impl From<threshold_crypto::SecretKey> for SecretKey {
    fn from(secret_key: threshold_crypto::SecretKey) -> Self {
        Self::Bls(secret_key)
    }
}

impl From<threshold_crypto::SecretKeyShare> for SecretKey {
    fn from(secret_key: threshold_crypto::SecretKeyShare) -> Self {
        Self::BlsShare(secret_key)
    }
}

impl Debug for SecretKey {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "SecretKey::")?;
        match self {
            Self::Ed25519(_) => write!(formatter, "Ed25519(..)"),
            Self::Bls(_) => write!(formatter, "Bls(..)"),
            Self::BlsShare(_) => write!(formatter, "BlsShare(..)"),
        }
    }
}

impl Display for SecretKey {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Debug::fmt(self, formatter)
    }
}

// TODO: implement Hash, Ord, PartialOrd for SecretKey?

/// Wrapper for different signature types.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum Signature {
    /// Ed25519 signature.
    Ed25519(ed25519_dalek::Signature),
    /// BLS signature.
    Bls(threshold_crypto::Signature),
    /// BLS signature share.
    BlsShare(threshold_crypto::SignatureShare),
}

impl From<threshold_crypto::Signature> for Signature {
    fn from(sig: threshold_crypto::Signature) -> Self {
        Self::Bls(sig)
    }
}

impl From<ed25519_dalek::Signature> for Signature {
    fn from(sig: ed25519_dalek::Signature) -> Self {
        Self::Ed25519(sig)
    }
}

impl From<threshold_crypto::SignatureShare> for Signature {
    fn from(sig: threshold_crypto::SignatureShare) -> Self {
        Self::BlsShare(sig)
    }
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for Signature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        utils::serialise(&self).hash(state)
    }
}

impl Ord for Signature {
    fn cmp(&self, other: &Signature) -> Ordering {
        utils::serialise(&self).cmp(&utils::serialise(other))
    }
}

impl PartialOrd for Signature {
    fn partial_cmp(&self, other: &Signature) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Debug for Signature {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "Signature::")?;
        match self {
            Self::Ed25519(_) => write!(formatter, "Ed25519(..)"),
            Self::Bls(_) => write!(formatter, "Bls(..)"),
            Self::BlsShare(_) => write!(formatter, "BlsShare(..)"),
        }
    }
}

/// Wrapper for different keypair types.
#[derive(Serialize, Deserialize)]
pub enum Keypair {
    /// Ed25519 keypair.
    Ed25519(ed25519_dalek::Keypair),
    /// BLS keypair.
    Bls(BlsKeypair),
    /// BLS keypair share.
    BlsShare(BlsKeypairShare),
}

// Need to manually implement this due to a missing impl in `Ed25519::Keypair`.
impl Clone for Keypair {
    fn clone(&self) -> Self {
        match self {
            Self::Ed25519(keypair) => Self::Ed25519(unwrap!(ed25519_dalek::Keypair::from_bytes(
                &keypair.to_bytes()
            ))),
            Self::Bls(keypair) => Self::Bls(keypair.clone()),
            Self::BlsShare(keypair) => Self::BlsShare(keypair.clone()),
        }
    }
}

impl Debug for Keypair {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "Keypair::")?;
        match self {
            Self::Ed25519(_) => write!(formatter, "Ed25519(..)"),
            Self::Bls(_) => write!(formatter, "Bls(..)"),
            Self::BlsShare(_) => write!(formatter, "BlsShare(..)"),
        }
    }
}

// Need to manually implement this due to a missing impl in `Ed25519::Keypair`.
impl PartialEq for Keypair {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ed25519(keypair), Self::Ed25519(other_keypair)) => {
                // TODO: After const generics land, remove the `to_vec()` calls.
                keypair.to_bytes().to_vec() == other_keypair.to_bytes().to_vec()
            }
            (Self::Bls(keypair), Self::Bls(other_keypair)) => keypair == other_keypair,
            (Self::BlsShare(keypair), Self::BlsShare(other_keypair)) => keypair == other_keypair,
            _ => false,
        }
    }
}

// Need to manually implement this due to a missing impl in `Ed25519::Keypair`.
impl Eq for Keypair {}

impl Keypair {
    /// Constructs a random Ed25519 public keypair.
    pub fn new_ed25519<T: CryptoRng + Rng>(rng: &mut T) -> Self {
        let keypair = ed25519_dalek::Keypair::generate::<Ed25519Digest, _>(rng);
        Self::Ed25519(keypair)
    }

    /// Constructs a random BLS public keypair.
    pub fn new_bls<T: CryptoRng + Rng>(rng: &mut T) -> Self {
        let bls_secret_key = rng.gen::<threshold_crypto::SecretKey>();
        let bls_public_key = bls_secret_key.public_key();
        let keypair = BlsKeypair {
            secret: SerdeSecret(bls_secret_key),
            public: bls_public_key,
        };
        Self::Bls(keypair)
    }

    /// Constructs a random BLS public keypair share.
    pub fn new_bls_share(bls_secret_key_share: threshold_crypto::SecretKeyShare) -> Self {
        let bls_public_key_share = bls_secret_key_share.public_key_share();
        let keypair_share = BlsKeypairShare {
            secret: SerdeSecret(bls_secret_key_share),
            public: bls_public_key_share,
        };
        Self::BlsShare(keypair_share)
    }

    /// Returns the public key associated with this keypair.
    pub fn public_key(&self) -> PublicKey {
        match self {
            Self::Ed25519(keypair) => PublicKey::Ed25519(keypair.public),
            Self::Bls(keypair) => PublicKey::Bls(keypair.public),
            Self::BlsShare(keypair) => PublicKey::BlsShare(keypair.public),
        }
    }
}

/// BLS keypair.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlsKeypair {
    /// Secret key.
    pub secret: SerdeSecret<threshold_crypto::SecretKey>,
    /// Public key.
    pub public: threshold_crypto::PublicKey,
}

/// BLS keypair share.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlsKeypairShare {
    /// Secret key share.
    pub secret: SerdeSecret<threshold_crypto::SecretKeyShare>,
    /// Public key share.
    pub public: threshold_crypto::PublicKeyShare,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils;
    use bincode::deserialize as deserialise;
    use rand;

    fn random_bls_secret_key() -> SecretKey {
        let mut rng = rand::thread_rng();
        SecretKey::new_bls(&mut rng)
    }

    fn random_bls_public_key() -> PublicKey {
        PublicKey::from(&random_bls_secret_key())
    }

    #[test]
    fn zbase32_encode_decode_public_key() {
        use unwrap::unwrap;

        let key = random_bls_public_key();
        assert_eq!(
            key,
            unwrap!(PublicKey::decode_from_zbase32(&key.encode_to_zbase32()))
        );
    }

    // Test serialising and deserialising public keys.
    #[test]
    fn serialisation_public_key() {
        let key = random_bls_public_key();
        let encoded = utils::serialise(&key);
        let decoded: PublicKey = unwrap!(deserialise(&encoded));

        assert_eq!(decoded, key);
    }

    // Test serialising and deserialising secret keys.
    #[test]
    fn serialisation_secret_key() {
        let key = random_bls_secret_key();
        let encoded = utils::serialise(&key);
        let decoded: SecretKey = unwrap!(deserialise(&encoded));

        assert_eq!(decoded, key);

        // TODO: test Ed25519 and BlsShare variants.
    }
}
