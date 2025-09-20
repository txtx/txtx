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