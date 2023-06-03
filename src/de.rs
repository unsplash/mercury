use serde::de::{Deserialize, Deserializer, Error};

pub fn only_true<'a, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'a>,
{
    bool::deserialize(deserializer).and_then(|b| {
        if b {
            Ok(b)
        } else {
            Err(Error::custom("invalid bool: false"))
        }
    })
}

#[test]
fn test_only_true() {
    #[derive(Debug, PartialEq, Eq, serde::Deserialize)]
    struct T {
        #[serde(deserialize_with = "only_true")]
        val: bool,
    }

    assert_eq!(
        serde_json::from_str::<T>(r#"{"val": true}"#).unwrap(),
        T { val: true },
    );

    assert!(serde_json::from_str::<T>(r#"{"val": false}"#).is_err());
}

pub fn only_false<'a, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'a>,
{
    bool::deserialize(deserializer).and_then(|b| {
        if b {
            Err(Error::custom("invalid bool: true"))
        } else {
            Ok(b)
        }
    })
}

#[test]
fn test_only_false() {
    #[derive(Debug, PartialEq, Eq, serde::Deserialize)]
    struct T {
        #[serde(deserialize_with = "only_false")]
        val: bool,
    }

    assert_eq!(
        serde_json::from_str::<T>(r#"{"val": false}"#).unwrap(),
        T { val: false },
    );

    assert!(serde_json::from_str::<T>(r#"{"val": true}"#).is_err());
}
