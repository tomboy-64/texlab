use crate::features::prelude::*;

pub fn label_name(table: &latex::SymbolTable, label: Option<&latex::Label>) -> Option<String> {
    label.map(|label| label.names(&table)[0].text().to_owned())
}

pub fn selection_range(
    table: &latex::SymbolTable,
    full_range: Range,
    label: Option<&latex::Label>,
) -> Range {
    label
        .map(|label| table[label.parent].range())
        .unwrap_or(full_range)
}
