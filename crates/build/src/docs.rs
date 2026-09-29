//! Splits rustdoc comments into the function, parameter and return descriptions agents read.

/// Rustdoc split into the parts the AppFunctions schema keeps apart.
#[derive(Debug, Default)]
pub struct Docs {
  pub description: Vec<String>,
  /// `(rust parameter name, description)` from an `# Arguments` / `# Parameters` section.
  pub params: Vec<(String, String)>,
  /// Contents of a `# Returns` section.
  pub returns: Option<String>,
}

enum Section {
  Description,
  Params,
  Returns,
}

impl Docs {
  pub fn parse(lines: &[String]) -> Self {
    let mut docs = Docs::default();
    let mut section = Section::Description;
    let mut returns = Vec::new();

    for line in lines {
      let trimmed = line.trim();
      if let Some(heading) = trimmed.strip_prefix('#') {
        let heading = heading.trim_start_matches('#').trim().to_ascii_lowercase();
        match heading.as_str() {
          "arguments" | "parameters" | "params" | "args" => {
            section = Section::Params;
            continue;
          }
          "returns" | "return" => {
            section = Section::Returns;
            continue;
          }
          _ => section = Section::Description,
        }
      }

      match section {
        Section::Description => docs.description.push(line.clone()),
        Section::Returns => {
          if !trimmed.is_empty() {
            returns.push(trimmed.to_owned());
          }
        }
        Section::Params => {
          if let Some((name, description)) = parse_param_bullet(trimmed) {
            docs.params.push((name, description));
          } else if !trimmed.is_empty()
            && let Some((_, description)) = docs.params.last_mut()
          {
            if !description.is_empty() {
              description.push(' ');
            }
            description.push_str(trimmed);
          }
        }
      }
    }

    while docs.description.last().is_some_and(|l| l.trim().is_empty()) {
      docs.description.pop();
    }
    while docs.description.first().is_some_and(|l| l.trim().is_empty()) {
      docs.description.remove(0);
    }
    if !returns.is_empty() {
      docs.returns = Some(returns.join(" "));
    }
    docs
  }

  pub fn param(&self, rust_name: &str) -> Option<&str> {
    self
      .params
      .iter()
      .find(|(name, _)| name == rust_name)
      .map(|(_, description)| description.as_str())
  }
}

/// Parses `* `name` - description`, `- name: description` and similar bullets.
fn parse_param_bullet(line: &str) -> Option<(String, String)> {
  let rest = line
    .strip_prefix('*')
    .or_else(|| line.strip_prefix('-'))?
    .trim_start();
  let (name, rest) = if let Some(rest) = rest.strip_prefix('`') {
    let end = rest.find('`')?;
    (&rest[..end], &rest[end + 1..])
  } else {
    let end = rest
      .find(|c: char| !(c.is_alphanumeric() || c == '_'))
      .unwrap_or(rest.len());
    (&rest[..end], &rest[end..])
  };
  if name.is_empty() {
    return None;
  }
  let description = rest
    .trim_start()
    .trim_start_matches(['-', ':', '–', '—'])
    .trim();
  Some((name.trim_start_matches("r#").to_owned(), description.to_owned()))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn lines(s: &str) -> Vec<String> {
    s.lines().map(str::to_owned).collect()
  }

  #[test]
  fn splits_sections() {
    let docs = Docs::parse(&lines(
      "Creates a task.\n\n# Arguments\n* `title` - The title.\n  Must not be empty.\n- content: Body text\n\n# Returns\nThe task.",
    ));
    assert_eq!(docs.description, vec!["Creates a task."]);
    assert_eq!(docs.param("title"), Some("The title. Must not be empty."));
    assert_eq!(docs.param("content"), Some("Body text"));
    assert_eq!(docs.returns.as_deref(), Some("The task."));
  }

  #[test]
  fn keeps_other_headings_in_description() {
    let docs = Docs::parse(&lines("Does things.\n\n# Errors\nFails sometimes."));
    assert_eq!(docs.description, vec!["Does things.", "", "# Errors", "Fails sometimes."]);
  }
}
