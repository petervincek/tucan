#[cfg(test)]
use ratatui::buffer::Buffer;

/// Transforms ratatui `Buffer` to `String`
#[cfg(test)]
pub fn buffer_to_string(buf: &Buffer) -> String {
    let area = buf.area;
    let mut lines = Vec::new();
    for y in 0..area.height {
        let mut line = String::new();
        for x in 0..area.width {
            line.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
        lines.push(line);
    }
    lines.join("\n")
}

/// Helps with assertion of the expected rendered output, to make it more visually understandable in the unit tests
#[cfg(test)]
pub fn assert_rendered_output(actual_output: &str, expected_output: &str) {
    if let Some(expected_output) = expected_output.strip_prefix("\n") {
        assert_eq!(
            actual_output, expected_output,
            "\nactual output:\n{actual_output}\nexpected output:\n{expected_output}"
        );
    } else {
        assert_eq!(
            actual_output, expected_output,
            "\nactual output:\n{actual_output}\nexpected output:\n{expected_output}"
        );
    }
}
