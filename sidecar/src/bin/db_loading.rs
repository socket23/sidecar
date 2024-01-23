//! We want to load from the codestory DB and then get all the prompts and the responses
//! which we get back

use std::sync::Arc;

use clap::Parser;
use sidecar::{
    application::{application::Application, config::configuration::Configuration},
    bg_poll::background_polling::poll_repo_updates,
    db::sqlite::{self, SqlDb},
    semantic_search::qdrant_process::{wait_for_qdrant, QdrantServerProcess},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let configuration = Arc::new(Configuration::parse());
    let sql_db = Arc::new(sqlite::init(configuration).await?);
    read_all_entries(sql_db).await?;
    Ok(())
}

async fn read_all_entries(sql_db: SqlDb) -> anyhow::Result<()> {
    let rows = sqlx::query! {
        "SELECT prompt, response FROM openai_llm_data WHERE event_type like \"%RequestAndResponse%\""
    }
    .fetch_all(sql_db.as_ref()).await.expect("to work");
    println!("{:?}", rows.len());
    rows.into_iter()
        .for_each(|row| match (row.prompt, row.response) {
            (Some(prompt), Some(response)) => {
                println!("Prompt: {}", prompt);
                println!("Response: {}", response);
            }
            _ => {}
        });
    Ok(())
}
