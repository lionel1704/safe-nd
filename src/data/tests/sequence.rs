// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// https://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use crate::data::{PrivateSequence, PublicSequence};
use crate::shared_types::Version;
use crate::XorName;
use unwrap::unwrap;

#[test]
fn append_public_data() {
    let mut data = PublicSequence::new(XorName([1; 32]), 10000);
    unwrap!(data.append(vec![b"hello".to_vec(), b"world".to_vec()], Some(0)));
}

#[test]
fn append_private_data() {
    let mut data = PrivateSequence::new(XorName(rand::random()), 10);

    // Assert that the values are appended.
    let values1 = vec![
        b"VALUE0".to_vec(),
        b"VALUE1".to_vec(),
        b"VALUE2".to_vec(),
        b"VALUE3".to_vec(),
    ];
    unwrap!(data.append(values1, Some(0)));
}

#[test]
fn in_range() {
    let mut data = PublicSequence::new(rand::random(), 10);
    let values = vec![
        b"value0".to_vec(),
        b"value1".to_vec(),
        b"value2".to_vec(),
        b"value3".to_vec(),
    ];
    unwrap!(data.append(values, Some(0)));

    assert_eq!(
        data.in_range(Version::FromStart(0), Version::FromStart(0)),
        Some(vec![b"value0".to_vec()])
    );
    assert_eq!(
        data.in_range(Version::FromStart(0), Version::FromStart(2)),
        Some(vec![
            b"value0".to_vec(),
            b"value1".to_vec(),
            b"value2".to_vec(),
        ])
    );
    assert_eq!(
        data.in_range(Version::FromStart(0), Version::FromStart(3)),
        Some(vec![
            b"value0".to_vec(),
            b"value1".to_vec(),
            b"value2".to_vec(),
            b"value3".to_vec(),
        ])
    );

    assert_eq!(
        data.in_range(Version::FromEnd(3), Version::FromEnd(2)),
        Some(vec![b"value0".to_vec(), b"value1".to_vec(),])
    );
    assert_eq!(
        data.in_range(Version::FromEnd(3), Version::FromEnd(0)),
        Some(vec![
            b"value0".to_vec(),
            b"value1".to_vec(),
            b"value2".to_vec(),
            b"value3".to_vec(),
        ])
    );

    assert_eq!(
        data.in_range(Version::FromStart(0), Version::FromEnd(0)),
        Some(vec![
            b"value0".to_vec(),
            b"value1".to_vec(),
            b"value2".to_vec(),
            b"value3".to_vec(),
        ])
    );

    // start > end
    assert_eq!(
        data.in_range(Version::FromStart(1), Version::FromStart(0)),
        None
    );
    assert_eq!(
        data.in_range(Version::FromEnd(1), Version::FromEnd(2)),
        None
    );

    // overflow
    assert_eq!(
        data.in_range(Version::FromStart(0), Version::FromStart(5)),
        None
    );
    assert_eq!(
        data.in_range(Version::FromEnd(5), Version::FromEnd(0)),
        None
    );
}
