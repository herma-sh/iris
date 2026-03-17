use super::Action;

/// Parses an OSC payload into terminal actions.
#[must_use]
pub fn parse_osc(payload: &[u8]) -> Vec<Action> {
    let Some(separator) = payload.iter().position(|&byte| byte == b';') else {
        return Vec::new();
    };

    let command = &payload[..separator];
    let data = &payload[(separator + 1)..];

    match command {
        b"0" | b"2" => String::from_utf8(data.to_vec())
            .ok()
            .map(|title| vec![Action::SetWindowTitle(title)])
            .unwrap_or_default(),
        b"8" => parse_hyperlink(data),
        _ => Vec::new(),
    }
}

fn parse_hyperlink(data: &[u8]) -> Vec<Action> {
    let mut parts = data.splitn(3, |&byte| byte == b';');
    let params = parts.next().unwrap_or_default();
    let uri = parts.next().unwrap_or_default();

    let uri = match String::from_utf8(uri.to_vec()) {
        Ok(uri) => uri,
        Err(_) => return Vec::new(),
    };

    let id = params
        .split(|&byte| byte == b':')
        .find_map(|part| part.strip_prefix(b"id="))
        .and_then(|value| String::from_utf8(value.to_vec()).ok());

    vec![Action::SetHyperlink { id, uri }]
}

#[cfg(test)]
mod tests {
    use super::parse_osc;
    use crate::parser::Action;

    #[test]
    fn osc_parses_window_titles() {
        assert_eq!(
            parse_osc(b"2;Iris"),
            vec![Action::SetWindowTitle("Iris".to_string())]
        );
    }

    #[test]
    fn osc_parses_hyperlinks() {
        assert_eq!(
            parse_osc(b"8;id=prompt-1;https://example.com"),
            vec![Action::SetHyperlink {
                id: Some("prompt-1".to_string()),
                uri: "https://example.com".to_string(),
            }]
        );
    }
}
