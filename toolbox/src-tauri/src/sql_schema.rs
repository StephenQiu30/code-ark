use crate::{
    contracts::ToolboxError,
    provisioner::{InitializationStep, fragments, identifier},
    workspace::error,
};
#[derive(Debug, PartialEq)]
pub struct Column {
    pub name: String,
    pub kind: String,
    pub nullable: bool,
    pub primary: bool,
}
pub fn expected_columns(step: &InitializationStep) -> Result<Vec<Column>, ToolboxError> {
    let begin = step
        .statement
        .find('(')
        .ok_or_else(|| error("InvalidSchema"))?;
    let end = step
        .statement
        .rfind(')')
        .ok_or_else(|| error("InvalidSchema"))?;
    let definitions = fragments(&step.statement[begin + 1..end], b',')?;
    let mut columns = vec![];
    let mut primary = vec![];
    for definition in &definitions {
        if definition.to_ascii_uppercase().starts_with("PRIMARY KEY") {
            let start = definition.find('(').ok_or_else(|| error("InvalidSchema"))?;
            let end = definition
                .rfind(')')
                .ok_or_else(|| error("InvalidSchema"))?;
            for key in fragments(&definition[start + 1..end], b',')? {
                let (name, tail) = identifier(&key)?;
                if !tail.trim().is_empty() {
                    return Err(error("InvalidSchema"));
                }
                primary.push(name);
            }
            continue;
        }
        if !definition.starts_with('`') {
            continue;
        }
        let (name, rest) = identifier(definition)?;
        let rest = rest.trim_start();
        let mut depth = 0;
        let mut end = rest.len();
        for (i, c) in rest.char_indices() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
            } else if c.is_whitespace() && depth == 0 {
                end = i;
                break;
            }
        }
        let kind = normalize_type(&rest[..end]);
        let nullable = !rest.to_ascii_uppercase().contains("NOT NULL");
        columns.push(Column {
            name,
            kind,
            nullable,
            primary: false,
        });
    }
    for c in &mut columns {
        if primary.contains(&c.name) {
            c.primary = true;
            c.nullable = false;
        }
    }
    if columns.is_empty() || primary.is_empty() {
        return Err(error("UnsupportedSchemaStructure"));
    }
    Ok(columns)
}
pub fn normalize_type(value: &str) -> String {
    let value = value.to_ascii_lowercase().replace(' ', "");
    if value == "integer" {
        "int".into()
    } else if value == "datetime(0)" {
        "datetime".into()
    } else {
        value
    }
}
pub fn seed_key(
    step: &InitializationStep,
    table: &InitializationStep,
) -> Result<(String, String), ToolboxError> {
    let columns = expected_columns(table)?;
    let primary = columns.iter().filter(|c| c.primary).collect::<Vec<_>>();
    if primary.len() != 1 {
        return Err(error("UnsupportedSeedKey"));
    }
    let (_, rest) = identifier(
        step.statement
            .strip_prefix("INSERT INTO ")
            .ok_or_else(|| error("InvalidSchema"))?,
    )?;
    let end = rest.find(')').ok_or_else(|| error("InvalidSchema"))?;
    let names = fragments(
        rest.trim_start()
            .strip_prefix('(')
            .ok_or_else(|| error("InvalidSchema"))?
            .split_once(')')
            .ok_or_else(|| error("InvalidSchema"))?
            .0,
        b',',
    )?;
    let position = names
        .iter()
        .position(|n| identifier(n).is_ok_and(|(n, _)| n == primary[0].name))
        .ok_or_else(|| error("UnsupportedSeedKey"))?;
    let row = rest[end + 1..].trim_start();
    let row = row
        .strip_prefix("VALUES ")
        .and_then(|r| r.strip_prefix('('))
        .and_then(|r| r.strip_suffix(')'))
        .ok_or_else(|| error("InvalidSchema"))?;
    let values = fragments(row, b',')?;
    let value = values
        .get(position)
        .ok_or_else(|| error("InvalidSchema"))?
        .clone();
    if !(value.bytes().all(|b| b.is_ascii_digit())
        || value.starts_with('\'') && value.ends_with('\''))
    {
        return Err(error("UnsupportedSeedKey"));
    }
    Ok((primary[0].name.clone(), value))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schema_observation_includes_types_nullability_and_primary_key() {
        let steps = crate::provisioner::parse_schema_steps(include_str!(
            "../resources/sql/seata_1_8_0_mysql.sql"
        ))
        .unwrap();
        let cols = expected_columns(&steps[0]).unwrap();
        assert_eq!(
            cols[0],
            Column {
                name: "xid".into(),
                kind: "varchar(128)".into(),
                nullable: false,
                primary: true
            }
        );
        let seed = steps.iter().find(|s| s.kind == "seed").unwrap();
        let table = steps
            .iter()
            .find(|s| s.object_name == seed.object_name && s.kind == "table")
            .unwrap();
        assert_eq!(
            seed_key(seed, table).unwrap(),
            ("lock_key".into(), "'AsyncCommitting'".into())
        );
    }
}
