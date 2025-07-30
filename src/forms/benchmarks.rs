use std::io::Read;

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use chrono::Utc;
use csv;
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;

use crate::domain::benchmark::NewBenchmark;
use crate::embedding::PromptEmbedding;

#[derive(Deserialize, Validate)]
pub struct AddBenchmarkForm {
    pub hub_id: i32,
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(length(min = 1))]
    pub sku: String,
    #[validate(length(min = 1))]
    pub category: String,
    #[validate(length(min = 1))]
    pub units: String,
    pub price: f64,
    pub amount: f64,
    #[validate(length(min = 1))]
    pub description: String,
}

impl PromptEmbedding for AddBenchmarkForm {
    fn prompt(&self) -> String {
        format!(
            "Name: {}\nSKU: {}\nCategory: {}\nUnits: {}\nPrice: {}\nAmount: {}\nDescription: {}",
            self.name,
            self.sku,
            self.category,
            self.units,
            self.price,
            self.amount,
            self.description
        )
    }
}

impl From<AddBenchmarkForm> for NewBenchmark {
    fn from(form: AddBenchmarkForm) -> Self {
        let embeddings = form.embeddings().unwrap_or_default();
        Self {
            hub_id: form.hub_id,
            name: form.name,
            sku: form.sku,
            category: form.category,
            units: form.units,
            price: form.price,
            amount: form.amount,
            description: form.description,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            embedding: embeddings,
        }
    }
}

#[derive(MultipartForm)]
pub struct UploadBenchmarksForm {
    #[multipart(limit = "10MB")]
    pub csv: TempFile,
}

#[derive(Debug, Error)]
pub enum UploadBenchmarksFormError {
    #[error("Error reading csv file")]
    FileReadError,
    #[error("Error parsing csv file")]
    CsvParseError,
}

impl From<std::io::Error> for UploadBenchmarksFormError {
    fn from(_: std::io::Error) -> Self {
        UploadBenchmarksFormError::FileReadError
    }
}

impl From<csv::Error> for UploadBenchmarksFormError {
    fn from(_: csv::Error) -> Self {
        UploadBenchmarksFormError::CsvParseError
    }
}

#[derive(Debug, Deserialize)]
struct CsvBenchmarkRow {
    pub name: String,
    pub sku: String,
    pub category: String,
    pub units: String,
    pub price: f64,
    pub amount: f64,
    pub description: String,
}

impl PromptEmbedding for CsvBenchmarkRow {
    fn prompt(&self) -> String {
        format!(
            "Name: {}\nSKU: {}\nCategory: {}\nUnits: {}\nPrice: {}\nAmount: {}\nDescription: {}",
            self.name,
            self.sku,
            self.category,
            self.units,
            self.price,
            self.amount,
            self.description
        )
    }
}

impl UploadBenchmarksForm {
    pub fn parse(&mut self, hub_id: i32) -> Result<Vec<NewBenchmark>, UploadBenchmarksFormError> {
        let mut csv_content = String::new();
        self.csv.file.read_to_string(&mut csv_content)?;

        let mut rdr = csv::Reader::from_reader(csv_content.as_bytes());

        let mut benchmarks = Vec::new();

        for result in rdr.deserialize::<CsvBenchmarkRow>() {
            let row = result?;

            let embedding = row.embeddings().unwrap_or_default();
            benchmarks.push(NewBenchmark {
                hub_id,
                name: row.name,
                sku: row.sku,
                category: row.category,
                units: row.units,
                price: row.price,
                amount: row.amount,
                description: row.description,
                created_at: Utc::now().naive_utc(),
                updated_at: Utc::now().naive_utc(),
                embedding,
            });
        }

        Ok(benchmarks)
    }
}
