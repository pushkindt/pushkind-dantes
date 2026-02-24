use serde::Serialize;
use thiserror::Error;

/// Row-level upload error used for UI reporting.
#[derive(Debug, Clone, Serialize)]
pub struct UploadRowError {
    pub row_number: usize,
    pub sku: Option<String>,
    pub message: String,
}

/// Aggregated upload outcome report.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UploadReport {
    pub total_rows: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<UploadRowError>,
}

impl UploadReport {
    pub fn with_total(total_rows: usize) -> Self {
        Self {
            total_rows,
            ..Self::default()
        }
    }

    pub fn push_error(
        &mut self,
        row_number: usize,
        sku: Option<String>,
        message: impl Into<String>,
    ) {
        self.skipped += 1;
        self.errors.push(UploadRowError {
            row_number,
            sku,
            message: message.into(),
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadFormat {
    Csv,
    Xlsx,
}

impl TryFrom<&str> for DownloadFormat {
    type Error = DownloadError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_ascii_lowercase().as_str() {
            "csv" => Ok(Self::Csv),
            "xlsx" => Ok(Self::Xlsx),
            other => Err(DownloadError::InvalidFormat(other.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadFile {
    pub file_name: String,
    pub content_type: &'static str,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("invalid download format: {0}")]
    InvalidFormat(String),
    #[error("failed to render csv")]
    CsvRender,
    #[error("failed to render xlsx")]
    XlsxRender,
}

pub fn render_download_file(
    base_name: &str,
    format: DownloadFormat,
    headers: &[&str],
    rows: &[Vec<String>],
) -> Result<DownloadFile, DownloadError> {
    match format {
        DownloadFormat::Csv => {
            let mut writer = csv::Writer::from_writer(vec![]);
            writer
                .write_record(headers)
                .map_err(|_| DownloadError::CsvRender)?;
            for row in rows {
                let escaped_row: Vec<String> =
                    row.iter().map(|value| escape_csv_cell(value)).collect();
                writer
                    .write_record(&escaped_row)
                    .map_err(|_| DownloadError::CsvRender)?;
            }
            let bytes = writer.into_inner().map_err(|_| DownloadError::CsvRender)?;
            Ok(DownloadFile {
                file_name: format!("{base_name}.csv"),
                content_type: "text/csv; charset=utf-8",
                bytes,
            })
        }
        DownloadFormat::Xlsx => {
            let mut workbook = rust_xlsxwriter::Workbook::new();
            let worksheet = workbook.add_worksheet();

            for (col_idx, header) in headers.iter().enumerate() {
                worksheet
                    .write_string(0, col_idx as u16, *header)
                    .map_err(|_| DownloadError::XlsxRender)?;
            }

            for (row_idx, row) in rows.iter().enumerate() {
                let sheet_row = (row_idx + 1) as u32;
                for (col_idx, value) in row.iter().enumerate() {
                    worksheet
                        .write_string(sheet_row, col_idx as u16, value)
                        .map_err(|_| DownloadError::XlsxRender)?;
                }
            }

            let bytes = workbook
                .save_to_buffer()
                .map_err(|_| DownloadError::XlsxRender)?;
            Ok(DownloadFile {
                file_name: format!("{base_name}.xlsx"),
                content_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                bytes,
            })
        }
    }
}

fn escape_csv_cell(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some('=' | '+' | '-' | '@') => format!("'{value}"),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{DownloadFormat, render_download_file};

    #[test]
    fn csv_export_escapes_formula_prefixed_cells() {
        let file = render_download_file(
            "products",
            DownloadFormat::Csv,
            &["sku", "url"],
            &[vec!["=SUM(A1:A2)".to_string(), "+malicious".to_string()]],
        )
        .expect("csv render should succeed");

        let csv_output = String::from_utf8(file.bytes).expect("csv output should be utf-8");
        assert!(csv_output.contains("'=SUM(A1:A2)"));
        assert!(csv_output.contains("'+malicious"));
    }

    #[test]
    fn csv_export_keeps_safe_cells_unchanged() {
        let file = render_download_file(
            "products",
            DownloadFormat::Csv,
            &["sku", "url"],
            &[vec![
                "SKU-123".to_string(),
                "https://example.com".to_string(),
            ]],
        )
        .expect("csv render should succeed");

        let csv_output = String::from_utf8(file.bytes).expect("csv output should be utf-8");
        assert!(csv_output.contains("SKU-123"));
        assert!(csv_output.contains("https://example.com"));
    }
}
