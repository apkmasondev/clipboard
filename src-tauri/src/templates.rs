use crate::model::{Result, MAX_TEXT};
use std::collections::BTreeMap;

// Templates are opt-in. No expressions, environment access or recursive expansion.
fn tokens(text: &str) -> Result<Vec<(usize, usize, String)>> {
    let mut out = Vec::new();
    let mut offset = 0;
    while let Some(start) = text[offset..].find("{{") {
        let start = offset + start;
        let end = text[start + 2..]
            .find("}}")
            .map(|n| start + 2 + n + 2)
            .ok_or("Pole szablonu musi kończyć się }}.")?;
        let name = text[start + 2..end - 2].trim();
        if name.is_empty()
            || name.chars().count() > 48
            || !name
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | ' '))
        {
            return Err("Nazwa pola: 1–48 liter, cyfr, spacji, znaków _ lub -.".into());
        }
        out.push((start, end, name.to_string()));
        if out.len() > 200 {
            return Err("Szablon może mieć maksymalnie 200 wystąpień pól.".into());
        }
        offset = end;
    }
    Ok(out)
}
pub fn fields(text: &str) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for (_, _, name) in tokens(text)? {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    if names.len() > 20 {
        return Err("Szablon może mieć maksymalnie 20 różnych pól.".into());
    }
    Ok(names)
}
pub fn render(text: &str, values: &BTreeMap<String, String>) -> Result<String> {
    let expected = fields(text)?;
    if values.keys().any(|k| !expected.contains(k)) {
        return Err("Formularz zawiera nieznane pole. Otwórz snippet ponownie.".into());
    }
    let mut result = String::new();
    let mut offset = 0;
    for (start, end, name) in tokens(text)? {
        let value = values
            .get(&name)
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| format!("Uzupełnij pole: {name}."))?;
        if result.len() + start - offset + value.len() > MAX_TEXT {
            return Err("Wynik szablonu przekracza 4 MB.".into());
        }
        result.push_str(&text[offset..start]);
        result.push_str(value);
        offset = end;
    }
    if result.len() + text.len() - offset > MAX_TEXT {
        return Err("Wynik szablonu przekracza 4 MB.".into());
    }
    result.push_str(&text[offset..]);
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_repetition_and_no_recursive_expansion() {
        let text = "Projekt {{ nazwa }}: {{żądanie}} / {{nazwa}}";
        assert_eq!(fields(text).unwrap(), vec!["nazwa", "żądanie"]);
        let values = BTreeMap::from([
            ("nazwa".into(), "{{sekret}}".into()),
            ("żądanie".into(), "Zażółć 🦀".into()),
        ]);
        assert_eq!(
            render(text, &values).unwrap(),
            "Projekt {{sekret}}: Zażółć 🦀 / {{sekret}}"
        );
    }
    #[test]
    fn malformed_missing_and_oversized_are_rejected() {
        for text in ["{{}}", "{{niezamknięte", "{{process.env.KEY}}"] {
            assert!(fields(text).is_err());
        }
        assert!(render("{{cel}}", &BTreeMap::new()).is_err());
        assert!(render(
            "{{cel}}{{cel}}",
            &BTreeMap::from([("cel".into(), "x".repeat(MAX_TEXT))])
        )
        .is_err());
        assert!(render("plain", &BTreeMap::from([("extra".into(), "x".into())])).is_err());
    }
}
