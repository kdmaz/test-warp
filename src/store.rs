use crate::types::{
    account::{Account, AccountId},
    answer::{Answer, AnswerId, NewAnswer},
    question::{NewQuestion, Question, QuestionId},
};
use handle_errors::Error;
use sqlx::{
    postgres::{PgPoolOptions, PgRow},
    PgPool, Row,
};

#[derive(Clone, Debug)]
pub struct Store {
    pub pool: PgPool,
}

impl Store {
    pub async fn new(connection_string: &str) -> Self {
        let pool = match PgPoolOptions::new()
            .max_connections(5)
            .connect(connection_string)
            .await
        {
            Ok(pool) => pool,
            Err(_) => panic!("Couldn't establish db connection"),
        };

        Store { pool }
    }

    pub async fn get_questions(
        &self,
        limit: Option<u32>,
        offset: u32,
    ) -> Result<Vec<Question>, Error> {
        match sqlx::query("SELECT * FROM questions LIMIT $1 OFFSET $2")
            .bind(limit)
            .bind(offset)
            .map(|row| Question {
                id: QuestionId(row.get("id")),
                title: row.get("title"),
                content: row.get("content"),
                tags: row.get("tags"),
            })
            .fetch_all(&self.pool)
            .await
        {
            Ok(questions) => Ok(questions),
            Err(e) => {
                tracing::event!(tracing::Level::ERROR, "{:?}", e);
                Err(Error::DatabaseQueryError(e))
            }
        }
    }

    pub async fn add_question(&self, question: NewQuestion) -> Result<Question, Error> {
        match sqlx::query(
            "INSERT INTO questions (title, content, tags)
            VALUES ($1, $2, $3)
            RETURNING id, title, content, tags",
        )
        .bind(question.title)
        .bind(question.content)
        .bind(question.tags)
        .map(|row| Question {
            id: QuestionId(row.get("id")),
            title: row.get("title"),
            content: row.get("content"),
            tags: row.get("tags"),
        })
        .fetch_one(&self.pool)
        .await
        {
            Ok(question) => Ok(question),
            Err(e) => {
                tracing::event!(tracing::Level::ERROR, "{:?}", e);
                Err(Error::DatabaseQueryError(e))
            }
        }
    }

    pub async fn update_question(&self, question: Question) -> Result<Question, Error> {
        match sqlx::query(
            "UPDATE questions
            SET
                title = $1,
                content = $2,
                tags = $3
            WHERE id = $4
            RETURNING id, title, content, tags",
        )
        .bind(question.title)
        .bind(question.content)
        .bind(question.tags)
        .bind(question.id.0)
        .map(|row| Question {
            id: QuestionId(row.get("id")),
            title: row.get("title"),
            content: row.get("content"),
            tags: row.get("tags"),
        })
        .fetch_one(&self.pool)
        .await
        {
            Ok(question) => Ok(question),
            Err(e) => {
                tracing::event!(tracing::Level::ERROR, "{:?}", e);
                Err(Error::DatabaseQueryError(e))
            }
        }
    }

    pub async fn delete_question(&self, id: i32) -> Result<bool, Error> {
        match sqlx::query("DELETE FROM questions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::event!(tracing::Level::ERROR, "{:?}", e);
                Err(Error::DatabaseQueryError(e))
            }
        }
    }

    pub async fn add_answer(&self, answer: NewAnswer) -> Result<Answer, Error> {
        match sqlx::query(
            "INSERT INTO answers (content, question_id) 
                VALUES ($1, $2)
                RETURNING id, content, question_id",
        )
        .bind(answer.content)
        .bind(answer.question_id.0)
        .map(|row| Answer {
            id: AnswerId(row.get("id")),
            content: row.get("content"),
            question_id: QuestionId(row.get("question_id")),
        })
        .fetch_one(&self.pool)
        .await
        {
            Ok(answer) => Ok(answer),
            Err(e) => {
                tracing::event!(tracing::Level::ERROR, "{:?}", e);
                Err(Error::DatabaseQueryError(e))
            }
        }
    }

    pub async fn add_account(self, account: Account) -> Result<bool, Error> {
        match sqlx::query("INSERT INTO accounts (email, password) VALUES ($1, $2)")
            .bind(account.email)
            .bind(account.password)
            .execute(&self.pool)
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::event!(
                    tracing::Level::ERROR,
                    code = e
                        .as_database_error()
                        .unwrap()
                        .code()
                        .unwrap()
                        .parse::<i32>()
                        .unwrap(),
                    db_message = e.as_database_error().unwrap().message(),
                    constraint = e.as_database_error().unwrap().constraint().unwrap()
                );
                Err(Error::DatabaseQueryError(e))
            }
        }
    }

    pub async fn get_account(self, email: String) -> Result<Account, Error> {
        match sqlx::query("SELECT * from accounts where email = $1")
            .bind(email)
            .map(|row: PgRow| Account {
                id: Some(AccountId(row.get("id"))),
                email: row.get("email"),
                password: row.get("password"),
            })
            .fetch_one(&self.pool)
            .await
        {
            Ok(account) => Ok(account),
            Err(e) => {
                tracing::event!(tracing::Level::ERROR, "{:?}", e);
                Err(Error::DatabaseQueryError(e))
            }
        }
    }
}
