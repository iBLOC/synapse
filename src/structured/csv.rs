use serde::de::DeserializeOwned;
use std::marker::PhantomData;

use crate::structured::{OutputError, OutputParser, OutputResult};

/// CSV 输出解析器
pub struct CsvOutputParser<T> {
    has_header: bool,
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned + Send + Sync> CsvOutputParser<T> {
    pub fn new() -> Self {
        Self {
            has_header: true,
            _phantom: PhantomData,
        }
    }

    pub fn without_header() -> Self {
        Self {
            has_header: false,
            _phantom: PhantomData,
        }
    }
}

impl<T: DeserializeOwned + Send + Sync> Default for CsvOutputParser<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: DeserializeOwned + Send + Sync> OutputParser<Vec<T>> for CsvOutputParser<T> {
    fn parse(&self, text: &str) -> OutputResult<Vec<T>> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(self.has_header)
            .from_reader(text.as_bytes());

        let mut results = Vec::new();
        for record in rdr.deserialize() {
            let record: T = record.map_err(|e| {
                OutputError::Parse(format!("CSV deserialize error: {e}"))
            })?;
            results.push(record);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Row {
        name: String,
        age: u32,
    }

    #[test]
    fn test_csv_parse() {
        let csv_text = "name,age\nAlice,30\nBob,25\n";
        let parser = CsvOutputParser::<Row>::new();
        let result = parser.parse(csv_text).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Alice");
    }
}