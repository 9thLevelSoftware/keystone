use crate::diagnostics::ValidationReport;
use crate::schema::PackRecord;

pub fn validate_pack(_pack: &PackRecord) -> ValidationReport {
    ValidationReport::new(Vec::new())
}
