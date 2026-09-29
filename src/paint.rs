//! Terminal styling that falls silent where it would be noise.
//!
//! Replaces the `colored` crate, whose MPL-2.0 would have obliged every
//! binary release to make its source available. crossterm, already linked, does the
//! styling; this module keeps `colored`'s method names and the two things
//! crossterm does not: no escape sequence at all when stdout is no terminal
//! or `NO_COLOR` is set — crossterm drops only the colours and still writes
//! bold and dim into a redirected log — and a width like `{:>12}` pads the
//! text, not the escape sequences.

use std::fmt;
use std::io::IsTerminal;
use std::sync::LazyLock;

pub use crossterm::style::Color;
use crossterm::style::{Attribute, ContentStyle, StyledContent};

static ENABLED: LazyLock<bool> = LazyLock::new(|| {
    std::io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
});

/// Text with a style, rendered plain where styling is off.
#[derive(Clone, Debug)]
pub struct Painted {
    text: String,
    style: ContentStyle,
}

impl fmt::Display for Painted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered = Rendered {
            painted: self,
            styled: *ENABLED,
        };
        fmt::Display::fmt(&rendered, f)
    }
}

/// One way of showing a `Painted` — the formatter's width applies to the text alone.
struct Rendered<'a> {
    painted: &'a Painted,
    styled: bool,
}

impl fmt::Display for Rendered<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let padded = pad(&self.painted.text, f);
        if self.styled {
            write!(
                f,
                "{}",
                StyledContent::new(self.painted.style, padded.as_str())
            )
        } else {
            f.write_str(&padded)
        }
    }
}

/// The text filled to the formatter's width and alignment — left by default, as for any string.
fn pad(text: &str, f: &fmt::Formatter<'_>) -> String {
    let length = text.chars().count();
    let Some(width) = f.width().filter(|width| *width > length) else {
        return text.to_string();
    };
    let gap = width - length;
    let (left, right) = match f.align() {
        Some(fmt::Alignment::Right) => (gap, 0),
        Some(fmt::Alignment::Center) => (gap / 2, gap - gap / 2),
        _ => (0, gap),
    };
    let fill = f.fill().to_string();
    format!("{}{text}{}", fill.repeat(left), fill.repeat(right))
}

/// `colored`'s names on crossterm's styles. The colours keep their ANSI codes:
/// `red` is 31, which crossterm calls `DarkRed`.
pub trait Paint {
    fn painted(&self) -> Painted;

    fn color(&self, color: Color) -> Painted {
        let mut painted = self.painted();
        painted.style.foreground_color = Some(color);
        painted
    }

    fn bold(&self) -> Painted {
        let mut painted = self.painted();
        painted.style.attributes.set(Attribute::Bold);
        painted
    }

    fn dimmed(&self) -> Painted {
        let mut painted = self.painted();
        painted.style.attributes.set(Attribute::Dim);
        painted
    }

    fn red(&self) -> Painted {
        self.color(Color::DarkRed)
    }

    fn green(&self) -> Painted {
        self.color(Color::DarkGreen)
    }

    fn yellow(&self) -> Painted {
        self.color(Color::DarkYellow)
    }

    fn blue(&self) -> Painted {
        self.color(Color::DarkBlue)
    }

    fn magenta(&self) -> Painted {
        self.color(Color::DarkMagenta)
    }

    fn cyan(&self) -> Painted {
        self.color(Color::DarkCyan)
    }

    fn bright_black(&self) -> Painted {
        self.color(Color::DarkGrey)
    }
}

impl Paint for str {
    fn painted(&self) -> Painted {
        Painted {
            text: self.to_string(),
            style: ContentStyle::new(),
        }
    }
}

impl Paint for Painted {
    fn painted(&self) -> Painted {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(painted: &Painted) -> Rendered<'_> {
        Rendered {
            painted,
            styled: false,
        }
    }

    fn styled(painted: &Painted) -> Rendered<'_> {
        Rendered {
            painted,
            styled: true,
        }
    }

    #[test]
    fn plain_output_carries_no_escape_sequence() {
        let painted = "│".red().dimmed().bold();

        assert_eq!(plain(&painted).to_string(), "│");
    }

    #[test]
    fn styled_output_wraps_the_text_in_escape_sequences() {
        let rendered = styled(&"gaffa".magenta()).to_string();

        assert!(rendered.starts_with('\u{1b}'), "{rendered:?}");
        assert!(rendered.contains("gaffa"));
    }

    #[test]
    fn a_chain_keeps_colour_and_attribute() {
        let painted = "│".red().dimmed();

        assert_eq!(painted.style.foreground_color, Some(Color::DarkRed));
        assert!(painted.style.attributes.has(Attribute::Dim));
    }

    #[test]
    fn a_width_pads_the_text_not_the_escape_sequences() {
        assert_eq!(format!("[{:>9}]", plain(&"RUNNING".green())), "[  RUNNING]");
        assert_eq!(format!("[{:<9}]", plain(&"web".cyan())), "[web      ]");
        assert_eq!(format!("[{:9}]", plain(&"web".cyan())), "[web      ]");
        assert!(format!("{:>9}", styled(&"RUNNING".green())).contains("  RUNNING"));
    }

    #[test]
    fn the_names_keep_the_ansi_codes_colored_used() {
        assert_eq!("x".cyan().style.foreground_color, Some(Color::DarkCyan));
        assert_eq!(
            "x".bright_black().style.foreground_color,
            Some(Color::DarkGrey)
        );
    }
}
