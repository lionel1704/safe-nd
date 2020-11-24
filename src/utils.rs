// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// https://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use crate::errors::convert_bincode_error;
use crate::{Error, Message, MessageId, PublicKey, Result, Signature};
use multibase::{self, Base};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Verify that a signature is valid for a given `Request` + `MessageId` combination.
pub fn verify_signature(
    signature: &Signature,
    public_key: &PublicKey,
    request: &Message,
    message_id: &MessageId,
) -> Result<()> {
    let message = serialise(&(request, *message_id))?;
    public_key.verify(signature, message)
}

/// Wrapper for raw bincode::serialise.
pub(crate) fn serialise<T: Serialize>(data: &T) -> Result<Vec<u8>> {
    bincode::serialize(data).map_err(convert_bincode_error)
}

/// Wrapper for bincode::deserialize.
pub(crate) fn deserialise<'a, T>(bytes: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    bincode::deserialize(bytes).map_err(convert_bincode_error)
}

/// Wrapper for z-Base-32 multibase::encode.
pub(crate) fn encode<T: Serialize>(data: &T) -> Result<String> {
    let bytes = serialise(&data)?;
    Ok(multibase::encode(Base::Base32Z, &bytes))
}

/// Wrapper for z-Base-32 multibase::decode.
pub(crate) fn decode<I: AsRef<str>, O: DeserializeOwned>(encoded: I) -> Result<O> {
    let (base, decoded) =
        multibase::decode(encoded).map_err(|e| Error::FailedToParse(e.to_string()))?;
    if base != Base::Base32Z {
        return Err(Error::FailedToParse(format!(
            "Expected z-base-32 encoding, but got {:?}",
            base
        )));
    }
    Ok(deserialise(&decoded).map_err(|e| Error::FailedToParse(e.to_string()))?)
}
