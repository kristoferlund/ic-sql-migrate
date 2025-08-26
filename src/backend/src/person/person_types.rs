use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Serialize, Deserialize, Default)]
pub struct PersonInput {
    pub name: String,
    pub age: u32,
}

#[derive(CandidType, Debug, Serialize, Deserialize, Default)]
pub struct Person {
    pub id: u32,
    pub name: String,
    pub age: u32,
}

#[derive(CandidType, Debug, Serialize, Deserialize, Default)]
pub struct QueryParams {
    pub limit: u32,
    pub offset: u32,
}

#[derive(CandidType, Debug, Serialize, Deserialize, Default)]
pub struct UpdateParams {
    pub id: u32,
    pub name: String,
}
