
fn collect_descendants(
    recipe: &AssetRecipe,
    instance: PartInstanceId,
    result: &mut BTreeSet<PartInstanceId>,
) {
    for child in recipe
        .instances
        .values()
        .filter(|candidate| candidate.parent == Some(instance))
    {
        if result.insert(child.id) {
            collect_descendants(recipe, child.id, result);
        }
    }
}

fn component_value(values: &[f32], component: &str, path: &str) -> Result<f32, AssetError> {
    let index = component_index(component, values.len(), path)?;
    values
        .get(index)
        .copied()
        .ok_or_else(|| AssetError::UnknownScalarPath(path.to_owned()))
}

fn set_component_value(
    values: &mut [f32],
    component: &str,
    path: &str,
    value: f32,
) -> Result<(), AssetError> {
    let index = component_index(component, values.len(), path)?;
    let Some(target) = values.get_mut(index) else {
        return Err(AssetError::UnknownScalarPath(path.to_owned()));
    };
    *target = value;
    Ok(())
}

fn scalar_to_u32(path: &str, value: f32) -> Result<u32, AssetError> {
    if !value.is_finite() {
        return Err(AssetError::NonFiniteScalar {
            path: path.to_owned(),
            value,
        });
    }
    if value < 0.0 {
        return Err(AssetError::InvalidScalarValue {
            path: path.to_owned(),
            value,
            reason: "value must not be negative",
        });
    }
    if value.fract() != 0.0 {
        return Err(AssetError::InvalidScalarValue {
            path: path.to_owned(),
            value,
            reason: "value must be an integer",
        });
    }
    if value > u32::MAX as f32 {
        return Err(AssetError::InvalidScalarValue {
            path: path.to_owned(),
            value,
            reason: "value exceeds u32 range",
        });
    }
    Ok(value as u32)
}

fn component_index(component: &str, length: usize, path: &str) -> Result<usize, AssetError> {
    let index = match component {
        "x" => 0,
        "y" => 1,
        "z" => 2,
        _ => return Err(AssetError::UnknownScalarPath(path.to_owned())),
    };
    if index < length {
        Ok(index)
    } else {
        Err(AssetError::UnknownScalarPath(path.to_owned()))
    }
}

fn parse_id(raw: &str, path: &str) -> Result<u64, AssetError> {
    raw.parse::<u64>()
        .map_err(|_| AssetError::UnknownScalarPath(path.to_owned()))
}

fn parse_index(raw: &str, path: &str) -> Result<usize, AssetError> {
    raw.parse::<usize>()
        .map_err(|_| AssetError::UnknownScalarPath(path.to_owned()))
}

fn append_subject(subject: Option<String>, suffix: &'static str) -> Option<String> {
    subject.map(|subject| format!("{subject}.{suffix}"))
}

fn definition_subject(definition: PartDefinitionId, suffix: &'static str) -> Option<String> {
    Some(format!("definition.{}.{suffix}", definition.0))
}

fn operation_subject(
    definition: PartDefinitionId,
    operation: OperationId,
    suffix: impl AsRef<str>,
) -> Option<String> {
    let suffix = suffix.as_ref();
    Some(format!(
        "definition.{}.operation.{}.{suffix}",
        definition.0, operation.0
    ))
}

fn array_is_finite(values: &[f32]) -> bool {
    values.iter().copied().all(f32::is_finite)
}

fn push_issue(
    report: &mut AssetValidationReport,
    subject: Option<String>,
    code: &'static str,
    message: impl Into<String>,
) {
    report.issues.push(AssetValidationIssue {
        subject,
        code: code.to_owned(),
        message: message.into(),
    });
}
