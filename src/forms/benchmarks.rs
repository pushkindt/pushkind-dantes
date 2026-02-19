use std::io::Read;

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use chrono::Utc;
use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::domain::benchmark::NewBenchmark;
use crate::domain::types::{
    BenchmarkId, BenchmarkName, BenchmarkSku, CategoryName, HubId, ProductAmount,
    ProductDescription, ProductId, ProductPrice, ProductUnits, TypeConstraintError,
};

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

/// Strongly-typed payload built from [`AddBenchmarkForm`].
#[derive(Debug, Clone, PartialEq)]
pub struct AddBenchmarkFormPayload {
    pub name: BenchmarkName,
    pub sku: BenchmarkSku,
    pub category: CategoryName,
    pub units: ProductUnits,
    pub price: ProductPrice,
    pub amount: ProductAmount,
    pub description: ProductDescription,
}

impl AddBenchmarkFormPayload {
    fn new(
        name: String,
        sku: String,
        category: String,
        units: String,
        price: f64,
        amount: f64,
        description: String,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self {
            name: BenchmarkName::new(name)?,
            sku: BenchmarkSku::new(sku)?,
            category: CategoryName::new(category)?,
            units: ProductUnits::new(units)?,
            price: ProductPrice::new(price)?,
            amount: ProductAmount::new(amount)?,
            description: ProductDescription::new(description)?,
        })
    }

    /// Construct a [`NewBenchmark`] domain model with contextual hub information.
    pub fn into_new_benchmark(self, hub_id: HubId) -> NewBenchmark {
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

/// Validation and conversion errors for [`AddBenchmarkForm`].
#[derive(Debug, Error)]
pub enum AddBenchmarkFormError {
    #[error("Add benchmark form validation failed: {0}")]
    Validation(String),
    #[error("Add benchmark form contains invalid data: {0}")]
    TypeConstraint(String),
}

impl From<ValidationErrors> for AddBenchmarkFormError {
    fn from(value: ValidationErrors) -> Self {
        AddBenchmarkFormError::Validation(value.to_string())
    }
}

impl From<TypeConstraintError> for AddBenchmarkFormError {
    fn from(value: TypeConstraintError) -> Self {
        AddBenchmarkFormError::TypeConstraint(value.to_string())
    }
}

impl TryFrom<AddBenchmarkForm> for AddBenchmarkFormPayload {
    type Error = AddBenchmarkFormError;

    fn try_from(value: AddBenchmarkForm) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(AddBenchmarkFormPayload::new(
            value.name,
            value.sku,
            value.category,
            value.units,
            value.price,
            value.amount,
            value.description,
        )?)
    }
}

/// Multipart form for uploading a CSV file with multiple benchmarks.
#[derive(MultipartForm, Validate)]
pub struct UploadBenchmarksForm {
    /// Uploaded CSV file containing benchmark rows.
    #[multipart(limit = "10MB")]
    pub csv: TempFile,
}

/// Strongly-typed payload built from [`UploadBenchmarksForm`].
#[derive(Debug, Clone, PartialEq)]
pub struct UploadBenchmarksFormPayload {
    pub benchmarks: Vec<AddBenchmarkFormPayload>,
}

impl UploadBenchmarksFormPayload {
    /// Construct [`NewBenchmark`] domain models with contextual hub information.
    pub fn into_new_benchmarks(self, hub_id: HubId) -> Vec<NewBenchmark> {
        self.benchmarks
            .into_iter()
            .map(|benchmark| benchmark.into_new_benchmark(hub_id))
            .collect()
    }
}

/// Errors that can occur while processing a [`UploadBenchmarksForm`].
#[derive(Debug, Error)]
pub enum UploadBenchmarksFormError {
    #[error("Upload benchmarks form validation failed: {0}")]
    Validation(String),
    /// Wrapper for I/O errors when reading the uploaded file.
    #[error("Error reading csv file")]
    FileReadError,
    /// The CSV content could not be parsed into benchmark records.
    #[error("Error parsing csv file")]
    CsvParseError,
    /// Parsed data violated domain type constraints.
    #[error("Invalid benchmark data: {0}")]
    TypeConstraint(String),
}

impl From<ValidationErrors> for UploadBenchmarksFormError {
    fn from(value: ValidationErrors) -> Self {
        UploadBenchmarksFormError::Validation(value.to_string())
    }
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

impl From<TypeConstraintError> for UploadBenchmarksFormError {
    fn from(value: TypeConstraintError) -> Self {
        UploadBenchmarksFormError::TypeConstraint(value.to_string())
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

impl TryFrom<&mut UploadBenchmarksForm> for UploadBenchmarksFormPayload {
    type Error = UploadBenchmarksFormError;

    fn try_from(value: &mut UploadBenchmarksForm) -> Result<Self, Self::Error> {
        value.validate()?;

        let mut csv_content = String::new();
        value.csv.file.read_to_string(&mut csv_content)?;

        let mut rdr = csv::Reader::from_reader(csv_content.as_bytes());
        let mut benchmarks = Vec::new();

        for result in rdr.deserialize::<CsvBenchmarkRow>() {
            let row = result?;
            benchmarks.push(AddBenchmarkFormPayload::new(
                row.name,
                row.sku,
                row.category,
                row.units,
                row.price,
                row.amount,
                row.description,
            )?);
        }

        Ok(Self { benchmarks })
    }
}

impl TryFrom<UploadBenchmarksForm> for UploadBenchmarksFormPayload {
    type Error = UploadBenchmarksFormError;

    fn try_from(mut value: UploadBenchmarksForm) -> Result<Self, Self::Error> {
        (&mut value).try_into()
    }
}

/// Form used to remove a benchmark association from a product.
#[derive(Deserialize, Validate)]
pub struct UnassociateForm {
    /// Benchmark identifier.
    #[validate(range(min = 1))]
    pub benchmark_id: i32,
    /// Product identifier.
    #[validate(range(min = 1))]
    pub product_id: i32,
}

/// Strongly-typed payload built from [`UnassociateForm`].
#[derive(Debug, Clone, PartialEq)]
pub struct UnassociateFormPayload {
    pub benchmark_id: BenchmarkId,
    pub product_id: ProductId,
}

/// Validation and conversion errors for [`UnassociateForm`].
#[derive(Debug, Error)]
pub enum UnassociateFormError {
    #[error("Unassociate form validation failed: {0}")]
    Validation(String),
    #[error("Unassociate form contains invalid data: {0}")]
    TypeConstraint(String),
}

impl From<ValidationErrors> for UnassociateFormError {
    fn from(value: ValidationErrors) -> Self {
        UnassociateFormError::Validation(value.to_string())
    }
}

impl From<TypeConstraintError> for UnassociateFormError {
    fn from(value: TypeConstraintError) -> Self {
        UnassociateFormError::TypeConstraint(value.to_string())
    }
}

impl TryFrom<UnassociateForm> for UnassociateFormPayload {
    type Error = UnassociateFormError;

    fn try_from(value: UnassociateForm) -> Result<Self, Self::Error> {
        value.validate()?;

        Ok(Self {
            benchmark_id: BenchmarkId::new(value.benchmark_id)?,
            product_id: ProductId::new(value.product_id)?,
        })
    }
}

/// Form used to create a benchmark association for a product.
#[derive(Deserialize, Validate)]
pub struct AssociateForm {
    /// Benchmark identifier.
    #[validate(range(min = 1))]
    pub benchmark_id: i32,
    /// Product identifier.
    #[validate(range(min = 1))]
    pub product_id: i32,
}

/// Strongly-typed payload built from [`AssociateForm`].
#[derive(Debug, Clone, PartialEq)]
pub struct AssociateFormPayload {
    pub benchmark_id: BenchmarkId,
    pub product_id: ProductId,
}

/// Validation and conversion errors for [`AssociateForm`].
#[derive(Debug, Error)]
pub enum AssociateFormError {
    #[error("Associate form validation failed: {0}")]
    Validation(String),
    #[error("Associate form contains invalid data: {0}")]
    TypeConstraint(String),
}

impl From<ValidationErrors> for AssociateFormError {
    fn from(value: ValidationErrors) -> Self {
        AssociateFormError::Validation(value.to_string())
    }
}

impl From<TypeConstraintError> for AssociateFormError {
    fn from(value: TypeConstraintError) -> Self {
        AssociateFormError::TypeConstraint(value.to_string())
    }
}

impl TryFrom<AssociateForm> for AssociateFormPayload {
    type Error = AssociateFormError;

    fn try_from(value: AssociateForm) -> Result<Self, Self::Error> {
        value.validate()?;

        Ok(Self {
            benchmark_id: BenchmarkId::new(value.benchmark_id)?,
            product_id: ProductId::new(value.product_id)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_benchmark_form_try_from_builds_payload() {
        let form = AddBenchmarkForm {
            name: "Bench".into(),
            sku: "SKU1".into(),
            category: "Fruit".into(),
            units: "kg".into(),
            price: 10.0,
            amount: 1.0,
            description: "Desc".into(),
        };

        let payload = AddBenchmarkFormPayload::try_from(form).unwrap();
        assert_eq!(payload.name, "Bench");
        assert_eq!(payload.price, 10.0);
    }

    #[test]
    fn unassociate_form_try_from_validates_ids() {
        let form = UnassociateForm {
            benchmark_id: 0,
            product_id: 1,
        };

        let err = UnassociateFormPayload::try_from(form).unwrap_err();
        assert!(matches!(err, UnassociateFormError::Validation(_)));
    }
}
