#![allow(dead_code, unused_variables, unused_imports)]

#[macro_use]
extern crate juniper;
#[macro_use]
extern crate maplit;

use assert_json_diff::assert_json_include;
use juniper::{Executor, FieldResult, Variables, ID};
use juniper_from_schema::{graphql_schema, graphql_schema_from_file};
use serde_json::{self, json, Value};

use std::collections::HashMap;

pub struct Context {
    db: Db,
}

impl juniper::Context for Context {}

struct Db {
    humans: HashMap<&'static str, character::Human>,
}

mod input {
    use super::*;

    graphql_schema_from_file!(
        "tests/schemas/complex_schema.graphql",
        with_idents: [ColorInput, ReviewInput]
    );
}

mod character {
    use super::*;

    graphql_schema_from_file!(
        "tests/schemas/complex_schema.graphql",
        with_idents: [Human, Droid, Character]
    );

    #[derive(Clone)]
    pub struct Human {
        pub id: &'static str,
        pub name: String,
    }

    impl HumanFields for Human {
        fn field_id<'a>(&self, executor: &Executor<'a, Context>) -> FieldResult<ID> {
            Ok(ID::new(self.id))
        }

        fn field_name<'a>(&self, executor: &Executor<'a, Context>) -> FieldResult<&String> {
            Ok(&self.name)
        }
    }

    #[derive(Clone)]
    pub struct Droid {
        id: &'static str,
        name: String,
    }

    impl DroidFields for Droid {
        fn field_id<'a>(&self, executor: &Executor<'a, Context>) -> FieldResult<ID> {
            Ok(ID::new(self.id))
        }

        fn field_name<'a>(&self, executor: &Executor<'a, Context>) -> FieldResult<&String> {
            Ok(&self.name)
        }
    }
}

mod episode {
    use super::*;
    graphql_schema_from_file!(
        "tests/schemas/complex_schema.graphql",
        with_idents: [Episode]
    );
}

mod search_result {
    use super::*;
    use character::{Droid, Human};

    graphql_schema_from_file!(
        "tests/schemas/complex_schema.graphql",
        with_idents: [SearchResult]
    );
}

mod review {
    use super::*;
    use episode::Episode;
    use input::ColorInput;

    graphql_schema_from_file!(
        "tests/schemas/complex_schema.graphql",
        with_idents: [Review]
    );

    pub struct Review {
        pub episode: Option<Episode>,
        pub stars: i32,
        pub commentary: Option<String>,
        pub favorite_color: Option<ColorInput>,
    }

    impl ReviewFields for Review {
        fn field_episode<'a>(
            &self,
            executor: &Executor<'a, Context>,
        ) -> FieldResult<&Option<Episode>> {
            Ok(&self.episode)
        }

        fn field_stars<'a>(&self, executor: &Executor<'a, Context>) -> FieldResult<&i32> {
            Ok(&self.stars)
        }

        fn field_commentary<'a>(
            &self,
            executor: &Executor<'a, Context>,
        ) -> FieldResult<&Option<String>> {
            Ok(&self.commentary)
        }

        fn field_favorite_color<'a>(
            &self,
            executor: &Executor<'a, Context>,
            _: &QueryTrail<'a, ColorInput, Walked>,
        ) -> FieldResult<&Option<ColorInput>> {
            Ok(&self.favorite_color)
        }
    }
}

mod query {
    use super::*;
    use character::Character;
    use episode::Episode;
    use search_result::SearchResult;

    graphql_schema_from_file!("tests/schemas/complex_schema.graphql", with_idents: [Query]);

    pub struct Query;

    impl QueryFields for Query {
        fn field_hero<'a>(
            &self,
            executor: &Executor<'a, Context>,
            trail: &QueryTrail<'a, Character, Walked>,
            episode: Option<Episode>,
        ) -> FieldResult<Option<Character>> {
            let hero = episode.and_then(|episode| {
                let luke = executor
                    .context()
                    .db
                    .humans
                    .get(&"1")
                    .map(|h| Character::from(h.clone()));

                match episode {
                    Episode::Newhope => luke,
                    Episode::Empire => luke,
                    Episode::Jedi => luke,
                }
            });

            Ok(hero)
        }

        fn field_search<'a>(
            &self,
            executor: &Executor<'a, Context>,
            trail: &QueryTrail<'a, SearchResult, Walked>,
            text: Option<String>,
        ) -> FieldResult<Option<Vec<SearchResult>>> {
            let results = text.map(|text| {
                executor
                    .context()
                    .db
                    .humans
                    .clone()
                    .into_iter()
                    .map(|(_, human)| human)
                    .filter(|human| human.name.contains(&text))
                    .map(SearchResult::from)
                    .collect::<Vec<_>>()
            });
            Ok(results)
        }
    }
}

mod mutation {
    use super::*;
    use episode::Episode;
    use input::ReviewInput;
    use review::Review;

    graphql_schema_from_file!(
        "tests/schemas/complex_schema.graphql",
        with_idents: [Mutation]
    );

    pub struct Mutation;

    impl MutationFields for Mutation {
        fn field_create_review<'a>(
            &self,
            executor: &Executor<'a, Context>,
            trail: &QueryTrail<'a, Review, Walked>,
            episode: Option<Episode>,
            review: ReviewInput,
        ) -> FieldResult<Option<Review>> {
            let review = Review {
                episode,
                stars: review.stars,
                commentary: review.commentary,
                favorite_color: review.favorite_color,
            };

            // the fact that everything type checks is test enough, we don't need to actually insert
            // this review

            Ok(Some(review))
        }
    }
}

use mutation::Mutation;
use query::Query;

graphql_schema_from_file!(
    "tests/schemas/complex_schema.graphql",
    with_idents: [schema]
);

#[test]
fn query_hero() {
    let value = run_query(r#"query { hero(episode: NEWHOPE) { id name } }"#);

    assert_json_include!(
        actual: value,
        expected: json!({
            "hero": {
                "id": "1",
                "name": "Luke Skywalker",
            }
        })
    );

    let value = run_query(r#"query { hero(episode: EMPIRE) { id name } }"#);

    assert_json_include!(
        actual: value,
        expected: json!({
            "hero": {
                "id": "1",
                "name": "Luke Skywalker",
            }
        })
    );

    let value = run_query(r#"query { hero(episode: JEDI) { id name } }"#);

    assert_json_include!(
        actual: value,
        expected: json!({
            "hero": {
                "id": "1",
                "name": "Luke Skywalker",
            }
        })
    );
}

#[test]
fn search() {
    let value = run_query(
        r#"
        query {
            search(text: "Luke") {
                ... on Human {
                    id name
                }
                ... on Droid {
                    id name
                }
            }
        }
        "#,
    );

    assert_json_include!(
        actual: value,
        expected: json!({
            "search": [
                { "id": "1", "name": "Luke Skywalker" },
            ]
        })
    );
}

fn run_query(query: &str) -> Value {
    use character::Human;

    let db = Db {
        humans: hashmap! {
            "1" => Human { id: "1", name: "Luke Skywalker".to_string() },
        },
    };

    let ctx = Context { db };

    let (res, _errors) = juniper::execute(
        query,
        None,
        &Schema::new(Query, Mutation),
        &Variables::new(),
        &ctx,
    )
    .unwrap();

    serde_json::from_str(&serde_json::to_string(&res).unwrap()).unwrap()
}
