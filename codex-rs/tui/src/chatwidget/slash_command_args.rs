//! Shared tokenization for slash commands with positional arguments and named settings.
//!
//! Values may follow their setting or use `=`, while switches never consume a value. A bare `--`
//! leaves all remaining tokens positional.

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct ParsedCommandArgs {
    pub(super) positionals: Vec<String>,
    named: Vec<NamedArgument>,
}

#[derive(Debug, PartialEq, Eq)]
struct NamedArgument {
    name: String,
    value: Option<String>,
}

impl ParsedCommandArgs {
    pub(super) fn value(&self, name: &str) -> Option<&str> {
        self.named
            .iter()
            .find(|argument| argument.name == name)
            .and_then(|argument| argument.value.as_deref())
    }
}

pub(super) fn parse(
    args: &str,
    value_settings: &[&str],
    switches: &[&str],
) -> Result<ParsedCommandArgs, String> {
    let mut words = shlex::split(args)
        .ok_or_else(|| "Invalid command arguments. Check the quoting.".to_string())?
        .into_iter()
        .peekable();
    let mut parsed = ParsedCommandArgs::default();
    let mut parse_settings = true;
    while let Some(word) = words.next() {
        if parse_settings && word == "--" {
            parse_settings = false;
        } else if parse_settings && word.starts_with("--") {
            let argument = parse_named_argument(&word, &mut words, value_settings, switches)?;
            if parsed
                .named
                .iter()
                .any(|existing| existing.name == argument.name)
            {
                return Err(format!(
                    "Command setting `{}` was provided more than once.",
                    argument.name
                ));
            }
            parsed.named.push(argument);
        } else {
            parsed.positionals.push(word);
        }
    }
    Ok(parsed)
}

fn parse_named_argument(
    word: &str,
    words: &mut std::iter::Peekable<std::vec::IntoIter<String>>,
    value_settings: &[&str],
    switches: &[&str],
) -> Result<NamedArgument, String> {
    let (name, inline_value) = word
        .split_once('=')
        .map_or((word, None), |(name, value)| (name, Some(value)));
    if value_settings.contains(&name) {
        let value = inline_value
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .or_else(|| words.next_if(|value| !value.starts_with("--")))
            .ok_or_else(|| format!("Command setting `{name}` requires a value."))?;
        return Ok(NamedArgument {
            name: name.to_string(),
            value: Some(value),
        });
    }
    if switches.contains(&name) {
        if inline_value.is_some() {
            return Err(format!("Command setting `{name}` does not take a value."));
        }
        return Ok(NamedArgument {
            name: name.to_string(),
            value: None,
        });
    }
    Err(format!("Unknown command setting `{name}`."))
}

#[cfg(test)]
#[path = "slash_command_args_tests.rs"]
mod tests;
