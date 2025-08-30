use crate::person::{
    person_manager::PersonManager,
    person_types::{Person, QueryParams},
};
use ic_cdk::query;

#[query]
pub fn person_query(params: QueryParams) -> Result<Vec<Person>, String> {
    PersonManager::query(params)
}
