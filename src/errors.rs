pub fn format_error<E: Into<anyhow::Error>>(error: E) -> String {
    let error = error.into();
    error
        .chain()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\nCaused by:\n    ")
}
