use candid::CandidType;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use validator_derive::Validate;

// #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
// #[serde(try_from = "String", into = "String")]
// pub struct Name(String);
//
// impl TryFrom<String> for Name {
//     type Error = String;
//     fn try_from(s: String) -> Result<Self, Self::Error> {
//         let n = s.chars().count();
//         if (3..=50).contains(&n) {
//             Ok(Name(s))
//         } else {
//             Err("name length must be 3..=50".into())
//         }
//     }
// }
// impl From<Name> for String {
//     fn from(n: Name) -> Self {
//         n.0
//     }
// }

#[derive(Serialize, Deserialize, Debug, CandidType, Validate)]
pub struct Person {
    pub id: Option<u32>,

    #[validate(length(min = 3, max = 50))]
    pub name: String,

    pub age: u32,
}

#[derive(Serialize, Deserialize, Debug, CandidType)]
pub struct PersonCreateDto {
    pub name: String,
    pub age: u32,
}

#[derive(Serialize, Deserialize, Debug, CandidType)]
pub struct QueryParams {
    pub limit: u32,
    pub offset: u32,
}

#[derive(Serialize, Deserialize, Debug, CandidType)]
pub struct UpdateParams {
    pub id: u32,
    pub name: String,
}
