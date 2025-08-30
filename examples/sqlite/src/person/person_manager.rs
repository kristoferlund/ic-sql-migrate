use ic_rusqlite::with_connection;

use super::person_types::{Person, PersonCreateDto, QueryParams, UpdateParams};

pub struct PersonManager {}

impl PersonManager {
    pub fn create(person: Person) -> Result<Person, String> {
        with_connection(|conn| {
            let person = conn
                .query_row(
                    "INSERT INTO person (name, age) VALUES (?1, ?2) RETURNING id, name, age;",
                    (&person.name, person.age),
                    |row| {
                        Ok(Person {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            age: row.get(2)?,
                        })
                    },
                )
                .map_err(|err| format!("{err}"))?;

            Ok(person)
        })
    }

    pub fn update(params: UpdateParams) -> Result<Person, String> {
        with_connection(|conn| {
            // Update the person
            conn.execute(
                "update person set name=?1 where id=?2",
                (&params.name, params.id),
            )
            .map_err(|err| format!("{err:?}"))?;

            // Then fetch and return the updated person
            let mut stmt = conn
                .prepare("select * from person where id=?1")
                .map_err(|err| format!("{err}"))?;

            let person = stmt
                .query_row((params.id,), |row| {
                    Ok(Person {
                        id: row.get(0).unwrap(),
                        name: row.get(1).unwrap(),
                        age: row.get(2).unwrap(),
                    })
                })
                .map_err(|err| format!("{err:?}"))?;

            Ok(person)
        })
    }

    pub fn delete(id: u32) -> Result<Person, String> {
        with_connection(|conn| {
            // First fetch the person to return it
            let mut stmt = conn
                .prepare("select * from person where id=?1")
                .map_err(|err| format!("{err}"))?;

            let person = stmt
                .query_row((id,), |row| {
                    Ok(Person {
                        id: row.get(0).unwrap(),
                        name: row.get(1).unwrap(),
                        age: row.get(2).unwrap(),
                    })
                })
                .map_err(|err| format!("{err:?}"))?;

            // Then delete it
            conn.execute("delete from person where id=?1", (id,))
                .map_err(|err| format!("{err:?}"))?;

            Ok(person)
        })
    }

    pub fn query(params: QueryParams) -> Result<Vec<Person>, String> {
        with_connection(|conn| {
            let mut stmt = conn
                .prepare("select * from person limit ?1 offset ?2")
                .map_err(|err| format!("{err}"))?;

            let person_iter = stmt
                .query_map((params.limit, params.offset), |row| {
                    Ok(Person {
                        id: row.get(0).unwrap(),
                        name: row.get(1).unwrap(),
                        age: row.get(2).unwrap(),
                    })
                })
                .map_err(|err| format!("{err:?}"))?;

            let mut persons = Vec::new();
            for person in person_iter {
                persons.push(person.unwrap());
            }
            Ok(persons)
        })
    }
}
