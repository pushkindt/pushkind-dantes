use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};

use actix_multipart::form::{MultipartForm, tempfile::TempFile, text::Text};
use calamine::{Data, Reader, open_workbook_auto};
use thiserror::Error;

const PRODUCTS_HEADERS: [&str; 8] = [
    "sku",
    "name",
    "category",
    "units",
    "price",
    "amount",
    "description",
    "url",
];

const BENCHMARK_HEADERS: [&str; 7] = [
    "sku",
    "name",
    "category",
    "units",
    "price",
    "amount",
    "description",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadTarget {
    CrawlerProducts,
    Benchmarks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadFormat {
    Csv,
    Xlsx,
}

impl TryFrom<&str> for UploadFormat {
    type Error = UploadParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_ascii_lowercase().as_str() {
            "csv" => Ok(Self::Csv),
            "xlsx" => Ok(Self::Xlsx),
            other => Err(UploadParseError::InvalidFormat(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadMode {
    Full,
    Partial,
}

impl TryFrom<&str> for UploadMode {
    type Error = UploadParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_ascii_lowercase().as_str() {
            "full" => Ok(Self::Full),
            "partial" => Ok(Self::Partial),
            other => Err(UploadParseError::InvalidMode(other.to_string())),
        }
    }
}

#[derive(MultipartForm)]
pub struct UploadImportForm {
    #[multipart(limit = "10MB")]
    pub file: TempFile,
    pub format: Text<String>,
    pub mode: Text<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUploadRow {
    pub row_number: usize,
    pub values: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUpload {
    pub format: UploadFormat,
    pub mode: UploadMode,
    pub headers: Vec<String>,
    pub rows: Vec<ParsedUploadRow>,
}

#[derive(Debug, Error)]
pub enum UploadParseError {
    #[error("invalid upload format: {0}")]
    InvalidFormat(String),
    #[error("invalid upload mode: {0}")]
    InvalidMode(String),
    #[error("uploaded file is missing")]
    MissingFile,
    #[error("uploaded file extension does not match selected format")]
    ExtensionMismatch,
    #[error("uploaded file content type does not match selected format")]
    ContentTypeMismatch,
    #[error("failed to read uploaded file")]
    ReadFailed,
    #[error("failed to parse CSV")]
    CsvParseFailed,
    #[error("failed to parse XLSX")]
    XlsxParseFailed,
    #[error("uploaded file has no worksheet")]
    XlsxMissingSheet,
    #[error("header validation failed: {0}")]
    HeaderValidation(String),
}

impl From<std::io::Error> for UploadParseError {
    fn from(_: std::io::Error) -> Self {
        Self::ReadFailed
    }
}

impl From<csv::Error> for UploadParseError {
    fn from(_: csv::Error) -> Self {
        Self::CsvParseFailed
    }
}

impl From<calamine::Error> for UploadParseError {
    fn from(_: calamine::Error) -> Self {
        Self::XlsxParseFailed
    }
}

pub fn parse_upload(
    form: &mut UploadImportForm,
    target: UploadTarget,
) -> Result<ParsedUpload, UploadParseError> {
    let format = UploadFormat::try_from(form.format.as_str())?;
    let mode = UploadMode::try_from(form.mode.as_str())?;

    validate_file_meta(form, format)?;

    let (headers, rows) = match format {
        UploadFormat::Csv => parse_csv_rows(form)?,
        UploadFormat::Xlsx => parse_xlsx_rows(form)?,
    };

    let normalized_headers = normalize_headers(headers)?;
    validate_headers(target, mode, &normalized_headers)?;

    let parsed_rows = rows
        .into_iter()
        .enumerate()
        .map(|(idx, row)| {
            let mut values = HashMap::new();
            for (col_idx, header) in normalized_headers.iter().enumerate() {
                let value = row.get(col_idx).cloned().unwrap_or_default();
                values.insert(header.clone(), value.trim().to_string());
            }
            ParsedUploadRow {
                row_number: idx + 2,
                values,
            }
        })
        .collect::<Vec<_>>();

    Ok(ParsedUpload {
        format,
        mode,
        headers: normalized_headers,
        rows: parsed_rows,
    })
}

fn expected_headers(target: UploadTarget) -> &'static [&'static str] {
    match target {
        UploadTarget::CrawlerProducts => &PRODUCTS_HEADERS,
        UploadTarget::Benchmarks => &BENCHMARK_HEADERS,
    }
}

fn normalize_headers(headers: Vec<String>) -> Result<Vec<String>, UploadParseError> {
    let normalized = headers
        .into_iter()
        .map(|header| header.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        return Err(UploadParseError::HeaderValidation(
            "missing header row".to_string(),
        ));
    }

    if normalized.iter().any(|header| header.is_empty()) {
        return Err(UploadParseError::HeaderValidation(
            "header contains empty column name".to_string(),
        ));
    }

    let mut seen = HashSet::new();
    for header in &normalized {
        if !seen.insert(header.clone()) {
            return Err(UploadParseError::HeaderValidation(format!(
                "duplicate header column: {header}"
            )));
        }
    }

    Ok(normalized)
}

fn validate_headers(
    target: UploadTarget,
    mode: UploadMode,
    headers: &[String],
) -> Result<(), UploadParseError> {
    let expected = expected_headers(target);
    let expected_set = expected.iter().copied().collect::<HashSet<_>>();
    let header_set = headers.iter().map(String::as_str).collect::<HashSet<_>>();

    match mode {
        UploadMode::Full => {
            if header_set != expected_set {
                return Err(UploadParseError::HeaderValidation(format!(
                    "full mode requires exact headers: {}",
                    expected.join(",")
                )));
            }
        }
        UploadMode::Partial => {
            if !header_set.contains("sku") {
                return Err(UploadParseError::HeaderValidation(
                    "partial mode requires sku column".to_string(),
                ));
            }

            for header in headers {
                if !expected_set.contains(header.as_str()) {
                    return Err(UploadParseError::HeaderValidation(format!(
                        "partial mode contains unsupported column: {header}"
                    )));
                }
            }
        }
    }

    Ok(())
}

fn validate_file_meta(
    form: &UploadImportForm,
    format: UploadFormat,
) -> Result<(), UploadParseError> {
    let Some(file_name) = form.file.file_name.as_ref() else {
        return Err(UploadParseError::MissingFile);
    };

    let extension_ok = match format {
        UploadFormat::Csv => file_name.to_ascii_lowercase().ends_with(".csv"),
        UploadFormat::Xlsx => file_name.to_ascii_lowercase().ends_with(".xlsx"),
    };

    if !extension_ok {
        return Err(UploadParseError::ExtensionMismatch);
    }

    if let Some(content_type) = form.file.content_type.as_ref() {
        let mime = content_type.essence_str();
        let content_type_ok = match format {
            UploadFormat::Csv => matches!(
                mime,
                "text/csv" | "application/csv" | "application/vnd.ms-excel"
            ),
            UploadFormat::Xlsx => {
                mime == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            }
        };

        if !content_type_ok {
            return Err(UploadParseError::ContentTypeMismatch);
        }
    }

    Ok(())
}

fn parse_csv_rows(
    form: &mut UploadImportForm,
) -> Result<(Vec<String>, Vec<Vec<String>>), UploadParseError> {
    let file = form.file.file.as_file_mut();
    file.seek(SeekFrom::Start(0))?;

    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::None)
        .from_reader(content.as_bytes());

    let headers = reader
        .headers()?
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        rows.push(record.iter().map(|s| s.to_string()).collect());
    }

    Ok((headers, rows))
}

fn parse_xlsx_rows(
    form: &mut UploadImportForm,
) -> Result<(Vec<String>, Vec<Vec<String>>), UploadParseError> {
    let path = form.file.file.path().to_path_buf();
    let mut workbook = open_workbook_auto(path)?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or(UploadParseError::XlsxMissingSheet)??;

    let mut iter = range.rows();
    let Some(header_row) = iter.next() else {
        return Err(UploadParseError::HeaderValidation(
            "missing header row".to_string(),
        ));
    };

    let headers = header_row.iter().map(cell_to_string).collect::<Vec<_>>();

    let mut rows = Vec::new();
    for row in iter {
        rows.push(row.iter().map(cell_to_string).collect::<Vec<_>>());
    }

    Ok((headers, rows))
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        _ => cell.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_full_mode_exact_headers_products() {
        let headers = vec![
            "sku",
            "name",
            "category",
            "units",
            "price",
            "amount",
            "description",
            "url",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();

        assert!(
            validate_headers(UploadTarget::CrawlerProducts, UploadMode::Full, &headers).is_ok()
        );
    }

    #[test]
    fn rejects_partial_without_sku() {
        let headers = vec!["name", "price"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();

        let err = validate_headers(UploadTarget::Benchmarks, UploadMode::Partial, &headers)
            .unwrap_err()
            .to_string();
        assert!(err.contains("requires sku"));
    }

    #[test]
    fn rejects_partial_with_unknown_column() {
        let headers = vec!["sku", "foo"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();

        let err = validate_headers(UploadTarget::Benchmarks, UploadMode::Partial, &headers)
            .unwrap_err()
            .to_string();
        assert!(err.contains("unsupported column"));
    }

    #[test]
    fn rejects_full_mode_with_missing_column() {
        let headers = vec![
            "sku",
            "name",
            "category",
            "units",
            "price",
            "amount",
            "description",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();

        let err = validate_headers(UploadTarget::CrawlerProducts, UploadMode::Full, &headers)
            .unwrap_err()
            .to_string();
        assert!(err.contains("exact headers"));
    }
}
