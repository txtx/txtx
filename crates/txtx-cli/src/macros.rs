#![allow(unused)]

/// Base macro for colorizing text
/// This macro handles the common logic for all color macros
#[allow(unused_macros)]
macro_rules! colorize_impl {
    ($color_expr:expr, $($arg:tt)*) => {
        {
            use atty::Stream;
            use ansi_term::Style;
            if atty::is(Stream::Stdout) {
                format!("{}", $color_expr.paint(format!($($arg)*)))
            } else {
                format!($($arg)*)
            }
        }
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! green {
    ($($arg:tt)*) => {
        colorize_impl!(ansi_term::Colour::Green.bold(), $($arg)*)
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! red {
    ($($arg:tt)*) => {
        colorize_impl!(ansi_term::Colour::Red.bold(), $($arg)*)
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! yellow {
    ($($arg:tt)*) => {
        colorize_impl!(ansi_term::Colour::Yellow.bold(), $($arg)*)
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! blue {
    ($($arg:tt)*) => {
        colorize_impl!(ansi_term::Colour::Cyan.bold(), $($arg)*)
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! purple {
    ($($arg:tt)*) => {
        colorize_impl!(ansi_term::Colour::Purple.bold(), $($arg)*)
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! black {
    ($($arg:tt)*) => {
        colorize_impl!(ansi_term::Colour::Fixed(244), $($arg)*)
    }
}

#[macro_export]
macro_rules! pluralize {
    ($value:expr, $word:expr) => {
        if $value > 1 {
            format!("{} {}s", $value, $word)
        } else {
            format!("{} {}", $value, $word)
        }
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! format_err {
    ($($arg:tt)*) => {
        format!("{} {}", red!("error:"), $($arg)*)
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! format_warn {
    ($($arg:tt)*) => {
        format!("{} {}", yellow!("warn:"), $($arg)*)
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! format_note {
    ($($arg:tt)*) => {
        format!("{} {}", blue!("note:"), $($arg)*)
    }
}

#[cfg(test)]
mod tests {
    // Helper to create a test scenario where we know if we're in a TTY
    fn is_tty() -> bool {
        atty::is(atty::Stream::Stdout)
    }

    // ANSI escape codes for reference
    const ANSI_GREEN: &str = "\x1b[32;1m";
    const ANSI_RED: &str = "\x1b[31;1m";
    const ANSI_YELLOW: &str = "\x1b[33;1m";
    const ANSI_CYAN: &str = "\x1b[36;1m";  // blue! macro uses Cyan
    const ANSI_PURPLE: &str = "\x1b[35;1m";
    const ANSI_GRAY: &str = "\x1b[38;5;244m";  // black! uses Fixed(244)
    const ANSI_RESET: &str = "\x1b[0m";
    const ANSI_BOLD: &str = "\x1b[1m";

    // Test that all color macros produce output containing the input text
    #[test]
    fn test_color_macros_contain_text() {
        // These should always contain the input text, regardless of TTY status
        assert!(green!("hello").contains("hello"));
        assert!(red!("error").contains("error"));
        assert!(yellow!("warning").contains("warning"));
        assert!(blue!("info").contains("info"));
        assert!(purple!("purple").contains("purple"));
        assert!(black!("debug").contains("debug"));
    }

    #[test]
    fn test_ansi_codes_presence() {
        // This test documents the expected behavior:
        // - In a TTY: output contains ANSI codes
        // - Not in a TTY (like in tests): output is plain text

        let green_out = green!("test");
        let red_out = red!("test");
        let yellow_out = yellow!("test");

        if is_tty() {
            // When running in a terminal, should have ANSI codes
            assert!(green_out.contains(ANSI_GREEN) || green_out.contains("\x1b["));
            assert!(red_out.contains(ANSI_RED) || red_out.contains("\x1b["));
            assert!(yellow_out.contains(ANSI_YELLOW) || yellow_out.contains("\x1b["));
            assert!(green_out.contains(ANSI_RESET));
        } else {
            // When not in a terminal (usual test case), should be plain text
            assert_eq!(green_out, "test");
            assert_eq!(red_out, "test");
            assert_eq!(yellow_out, "test");
            assert!(!green_out.contains("\x1b"));
            assert!(!red_out.contains("\x1b"));
        }
    }

    // Test the colorize_impl macro directly with known colors
    #[test]
    fn test_colorize_impl_logic() {
        // We can test the macro expansion logic
        macro_rules! test_colorize {
            ($color:expr, $text:literal) => {{
                use atty::Stream;
                use ansi_term::Style;
                if atty::is(Stream::Stdout) {
                    format!("{}", $color.paint($text))
                } else {
                    format!($text)
                }
            }};
        }

        let result = test_colorize!(ansi_term::Colour::Green.bold(), "test");
        if is_tty() {
            // Should contain ANSI codes
            assert!(result.contains("test"));
            assert!(result.len() > 4); // "test" plus ANSI codes
        } else {
            // Should be plain text
            assert_eq!(result, "test");
        }
    }

    #[test]
    fn test_color_macros_with_formatting() {
        let value = 42;
        let colored = green!("Value: {}", value);
        assert!(colored.contains("Value: 42"));

        let multi = red!("{} {}", "error", 123);
        assert!(multi.contains("error 123"));
    }

    #[test]
    fn test_format_macros() {
        let err = format_err!("something went wrong");
        assert!(err.contains("error:"));
        assert!(err.contains("something went wrong"));

        let warn = format_warn!("deprecation warning");
        assert!(warn.contains("warn:"));
        assert!(warn.contains("deprecation warning"));

        let note = format_note!("additional info");
        assert!(note.contains("note:"));
        assert!(note.contains("additional info"));
    }

    #[test]
    fn test_pluralize_macro() {
        assert_eq!(pluralize!(1, "file"), "1 file");
        assert_eq!(pluralize!(0, "file"), "0 file");
        assert_eq!(pluralize!(2, "file"), "2 files");
        assert_eq!(pluralize!(100, "error"), "100 errors");
    }

    // Test that macros work with different input types
    #[test]
    fn test_macros_with_string_types() {
        let string = String::from("owned string");
        let slice = "string slice";

        // Should work with both &str and String
        assert!(green!("{}", string).contains("owned string"));
        assert!(green!("{}", slice).contains("string slice"));
        assert!(green!("literal").contains("literal"));
    }

    // Test ANSI code generation directly
    #[test]
    fn test_ansi_code_generation() {
        use ansi_term::Colour;
        use ansi_term::Style;

        // Test directly with ansi_term to verify our expectations
        let green_text = Colour::Green.bold().paint("test").to_string();
        let red_text = Colour::Red.bold().paint("test").to_string();
        let cyan_text = Colour::Cyan.bold().paint("test").to_string();
        let gray_text = Colour::Fixed(244).paint("test").to_string();

        // These should always have ANSI codes
        assert!(green_text.contains("\x1b["));
        assert!(green_text.contains("test"));
        assert!(green_text.contains("32")); // Green color code

        assert!(red_text.contains("\x1b["));
        assert!(red_text.contains("31")); // Red color code

        assert!(cyan_text.contains("\x1b["));
        assert!(cyan_text.contains("36")); // Cyan color code

        assert!(gray_text.contains("\x1b["));
        assert!(gray_text.contains("244")); // Fixed color 244
    }

    // Test that our macros behave consistently with the TTY detection
    #[test]
    fn test_macro_tty_behavior() {
        // Get the actual outputs
        let green_out = green!("test");
        let has_ansi = green_out.contains("\x1b[");

        // Check consistency - either all have ANSI or none do
        assert_eq!(has_ansi, red!("test").contains("\x1b["));
        assert_eq!(has_ansi, yellow!("test").contains("\x1b["));
        assert_eq!(has_ansi, blue!("test").contains("\x1b["));
        assert_eq!(has_ansi, purple!("test").contains("\x1b["));
        assert_eq!(has_ansi, black!("test").contains("\x1b["));

        // Verify the decision matches TTY status
        assert_eq!(has_ansi, is_tty());
    }
}
