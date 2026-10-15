use super::*;
use pretty_assertions::assert_eq;

#[test]
fn parses_positional_values_switches_and_quoted_arguments() {
    let parsed = parse(
        "'team review' item --reviewer user --speed=fast --clear -- --literal",
        &["--reviewer", "--speed"],
        &["--clear"],
    )
    .expect("valid command arguments");

    assert_eq!(
        parsed,
        ParsedCommandArgs {
            positionals: vec!["team review".into(), "item".into(), "--literal".into()],
            named: vec![
                NamedArgument {
                    name: "--reviewer".into(),
                    value: Some("user".into()),
                },
                NamedArgument {
                    name: "--speed".into(),
                    value: Some("fast".into()),
                },
                NamedArgument {
                    name: "--clear".into(),
                    value: None,
                },
            ],
        }
    );
    assert_eq!(parsed.value("--reviewer"), Some("user"));
}

#[test]
fn rejects_invalid_named_arguments() {
    for (args, expected) in [
        (
            "--reviewer",
            "Command setting `--reviewer` requires a value.",
        ),
        (
            "--clear=yes",
            "Command setting `--clear` does not take a value.",
        ),
        (
            "--clear --clear",
            "Command setting `--clear` was provided more than once.",
        ),
        ("--unknown", "Unknown command setting `--unknown`."),
        (
            "'unfinished",
            "Invalid command arguments. Check the quoting.",
        ),
    ] {
        assert_eq!(
            parse(args, &["--reviewer"], &["--clear"]),
            Err(expected.to_string())
        );
    }
}
