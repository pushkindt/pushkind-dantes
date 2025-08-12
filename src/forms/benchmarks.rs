use std::io::Read;

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use chrono::Utc;
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;

use pushkind_common::domain::benchmark::NewBenchmark;

/// Form data for creating a single benchmark item via the UI.
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
    /// Convert the validated form into a [`NewBenchmark`] domain model.
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

/// Multipart form for uploading a CSV file with multiple benchmarks.
#[derive(MultipartForm)]
pub struct UploadBenchmarksForm {
    /// Uploaded CSV file containing benchmark rows.
    #[multipart(limit = "10MB")]
    pub csv: TempFile,
}

/// Errors that can occur while processing a [`UploadBenchmarksForm`].
#[derive(Debug, Error)]
pub enum UploadBenchmarksFormError {
    /// Wrapper for I/O errors when reading the uploaded file.
    #[error("Error reading csv file")]
    FileReadError,
    /// The CSV content could not be parsed into benchmark records.
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
    /// Parse the uploaded CSV file into a list of [`NewBenchmark`] records.
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

/// Form used to remove a benchmark association from a product.
#[derive(Deserialize)]
pub struct UnassociateForm {
    /// Benchmark identifier.
    pub benchmark_id: i32,
    /// Product identifier.
    pub product_id: i32,
}

/// Form used to create a benchmark association for a product.
#[derive(Deserialize)]
pub struct AssociateForm {
    /// Benchmark identifier.
    pub benchmark_id: i32,
    /// Product identifier.
    pub product_id: i32,
}
