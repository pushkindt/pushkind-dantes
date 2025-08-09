use std::io::Read;

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use chrono::Utc;
use csv;
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;

use pushkind_common::domain::benchmark::NewBenchmark;

#[derive(Deserialize, Validate)]
pub struct AddBenchmarkForm {
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

impl AddBenchmarkForm {
    pub fn into_new_benchmark(self, hub_id: i32) -> NewBenchmark {
        let now = Utc::now().naive_utc();
        NewBenchmark {
            hub_id,
            name: self.name,
            sku: self.sku,
            category: self.category,
            units: self.units,
            price: self.price,
            amount: self.amount,
            description: self.description,
            created_at: now,
            updated_at: now,
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

impl UploadBenchmarksForm {
    pub fn parse(&mut self, hub_id: i32) -> Result<Vec<NewBenchmark>, UploadBenchmarksFormError> {
        let mut csv_content = String::new();
        self.csv.file.read_to_string(&mut csv_content)?;

        let mut rdr = csv::Reader::from_reader(csv_content.as_bytes());

        let mut benchmarks = Vec::new();

        for result in rdr.deserialize::<CsvBenchmarkRow>() {
            let row = result?;

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
            });
        }

        Ok(benchmarks)
    }
}

#[derive(Deserialize)]
pub struct UnassociateForm {
    pub benchmark_id: i32,
    pub product_id: i32,
}

#[derive(Deserialize)]
pub struct AssociateForm {
    pub benchmark_id: i32,
    pub product_id: i32,
}
