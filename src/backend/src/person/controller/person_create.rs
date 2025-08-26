use crate::person::{
    person_manager::PersonManager,
    person_types::{Person, PersonInput},
};
use ic_cdk::update;

#[update]
pub fn person_create(person: PersonInput) -> Result<Person, String> {
    PersonManager::create(person)
}
