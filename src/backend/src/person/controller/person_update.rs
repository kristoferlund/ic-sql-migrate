use crate::person::{
    person_manager::PersonManager,
    person_types::{Person, UpdateParams},
};
use ic_cdk::update;

#[update]
pub fn person_update(params: UpdateParams) -> Result<Person, String> {
    PersonManager::update(params)
}
