//! Harmonic mixing helpers: reading a musical key in any notation and telling how two keys relate
//! on the Camelot wheel, plus the pitch-range check DJs use for BPM.

/// A position on the Camelot wheel: a number from 1 to 12 and a mode (A = minor, B = major).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CamelotKey {
    pub number: u8,
    pub minor: bool,
}

/// How a second key sits relative to the first on the wheel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarmonicRelation {
    /// The same key.
    Same,
    /// One step away on the wheel, same mode (8A to 7A or 9A).
    Adjacent,
    /// The relative major or minor: same number, other mode (8A to 8B).
    Relative,
    /// Not a harmonic neighbour.
    Clash,
    /// At least one of the two keys is missing or unreadable.
    Unknown,
}

/// Note names of the wheel, `(number, is_minor)` by pitch class of the tonic (0 = C).
/// Majors: C 8B, G 9B, D 10B, A 11B, E 12B, B 1B, F# 2B, Db 3B, Ab 4B, Eb 5B, Bb 6B, F 7B.
/// Minors: A 8A, E 9A, B 10A, F# 11A, C# 12A, G# 1A, D# 2A, Bb 3A, F 4A, C 5A, G 6A, D 7A.
const MAJOR_BY_PITCH_CLASS: [u8; 12] = [8, 3, 10, 5, 12, 7, 2, 9, 4, 11, 6, 1];
const MINOR_BY_PITCH_CLASS: [u8; 12] = [5, 12, 7, 2, 9, 4, 11, 6, 1, 8, 3, 10];

impl CamelotKey {
    /// Reads a key written on the Camelot wheel (`8A`, `08a`, `8A - Energy 7`) or in musical
    /// notation (`Am`, `A minor`, `F#m`, `Bb`, `Db major`, `E♭ minor`). `None` when it is neither.
    pub fn parse(key: &str) -> Option<Self> {
        let text = key.trim();
        if text.is_empty() {
            return None;
        }
        Self::parse_camelot(text).or_else(|| Self::parse_musical(text))
    }

    fn parse_camelot(text: &str) -> Option<Self> {
        let digits: String = text.chars().take_while(|c| c.is_ascii_digit()).collect();
        let number: u8 = digits.parse().ok()?;
        if !(1..=12).contains(&number) {
            return None;
        }
        let mode = text[digits.len()..].chars().next()?;
        // `8A`, `8A (A minor)`, `8A - Energy 7`: the mode letter stands alone, not in a word.
        let after = text[digits.len() + mode.len_utf8()..].chars().next();
        if after.is_some_and(|c| c.is_alphanumeric()) {
            return None;
        }
        match mode.to_ascii_uppercase() {
            'A' => Some(Self {
                number,
                minor: true,
            }),
            'B' => Some(Self {
                number,
                minor: false,
            }),
            _ => None,
        }
    }

    fn parse_musical(text: &str) -> Option<Self> {
        let mut chars = text.chars().peekable();
        let letter = chars.next()?.to_ascii_uppercase();
        let natural: i32 = match letter {
            'C' => 0,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' => 11,
            _ => return None,
        };
        let mut pitch_class = natural;
        if let Some(&c) = chars.peek() {
            match c {
                '#' | '♯' => {
                    pitch_class += 1;
                    chars.next();
                }
                'b' | '♭' => {
                    pitch_class -= 1;
                    chars.next();
                }
                _ => {}
            }
        }
        let rest: String = chars.collect::<String>().trim().to_ascii_lowercase();
        let minor = match rest.as_str() {
            "" | "maj" | "major" => false,
            "m" | "min" | "minor" => true,
            _ => return None,
        };
        let pitch_class = pitch_class.rem_euclid(12) as usize;
        let table = if minor {
            &MINOR_BY_PITCH_CLASS
        } else {
            &MAJOR_BY_PITCH_CLASS
        };
        Some(Self {
            number: table[pitch_class],
            minor,
        })
    }

    /// How `other` relates to `self`.
    pub fn relation_to(self, other: Self) -> HarmonicRelation {
        if self == other {
            return HarmonicRelation::Same;
        }
        if self.number == other.number {
            return HarmonicRelation::Relative;
        }
        let step = (i16::from(self.number) - i16::from(other.number)).rem_euclid(12);
        if self.minor == other.minor && (step == 1 || step == 11) {
            return HarmonicRelation::Adjacent;
        }
        HarmonicRelation::Clash
    }
}

/// The relation between two keys given as free text; `Unknown` if either cannot be read.
pub fn relation(from: Option<&str>, to: Option<&str>) -> HarmonicRelation {
    match (
        from.and_then(CamelotKey::parse),
        to.and_then(CamelotKey::parse),
    ) {
        (Some(a), Some(b)) => a.relation_to(b),
        _ => HarmonicRelation::Unknown,
    }
}

/// Gap between two tempos as a percentage of the first (positive when `to` is faster).
pub fn bpm_delta_percent(from: Option<f64>, to: Option<f64>) -> Option<f64> {
    match (from, to) {
        (Some(a), Some(b)) if a > 0.0 && b > 0.0 => Some((b - a) / a * 100.0),
        _ => None,
    }
}

/// The pitch range of a standard DJ deck: a tempo within this many percent can be matched.
pub const PITCH_RANGE_PERCENT: f64 = 6.0;

#[cfg(test)]
mod tests {
    use super::*;

    fn key(text: &str) -> CamelotKey {
        CamelotKey::parse(text).unwrap_or_else(|| panic!("{text} should parse"))
    }

    #[test]
    fn camelot_notation_in_its_usual_spellings() {
        for text in ["8A", "08a", " 8A ", "8A - Energy 7", "8A (A minor)"] {
            assert_eq!(
                key(text),
                CamelotKey {
                    number: 8,
                    minor: true
                },
                "{text}"
            );
        }
        assert_eq!(
            key("12B"),
            CamelotKey {
                number: 12,
                minor: false
            }
        );
        assert_eq!(
            key("1b"),
            CamelotKey {
                number: 1,
                minor: false
            }
        );
    }

    #[test]
    fn musical_notation_maps_onto_the_wheel() {
        let cases = [
            ("Am", "8A"),
            ("A minor", "8A"),
            ("C", "8B"),
            ("C major", "8B"),
            ("G", "9B"),
            ("F#m", "11A"),
            ("Gbm", "11A"),
            ("F#", "2B"),
            ("Bb", "6B"),
            ("Bbm", "3A"),
            ("A#m", "3A"),
            ("Db major", "3B"),
            ("C#m", "12A"),
            ("E♭ minor", "2A"),
            ("Dm", "7A"),
            ("Fm", "4A"),
            ("Abm", "1A"),
            ("B", "1B"),
        ];
        for (musical, camelot) in cases {
            assert_eq!(key(musical), key(camelot), "{musical} should be {camelot}");
        }
    }

    #[test]
    fn text_that_is_not_a_key_is_rejected() {
        for text in [
            "", "  ", "13A", "0A", "8C", "H", "Am7", "x", "8Abc", "10", "-",
        ] {
            assert_eq!(CamelotKey::parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn the_wheel_wraps_around() {
        assert_eq!(
            key("12A").relation_to(key("1A")),
            HarmonicRelation::Adjacent
        );
        assert_eq!(
            key("1B").relation_to(key("12B")),
            HarmonicRelation::Adjacent
        );
        assert_eq!(key("8A").relation_to(key("7A")), HarmonicRelation::Adjacent);
        assert_eq!(key("8A").relation_to(key("9A")), HarmonicRelation::Adjacent);
    }

    #[test]
    fn same_relative_and_clashing_keys() {
        assert_eq!(key("8A").relation_to(key("8A")), HarmonicRelation::Same);
        assert_eq!(key("8A").relation_to(key("8B")), HarmonicRelation::Relative);
        assert_eq!(
            key("8A").relation_to(key("9B")),
            HarmonicRelation::Clash,
            "different number and mode"
        );
        assert_eq!(
            key("8A").relation_to(key("10A")),
            HarmonicRelation::Clash,
            "two steps away"
        );
    }

    #[test]
    fn relation_handles_missing_and_unreadable_keys() {
        assert_eq!(
            relation(Some("8A"), Some("Em")),
            HarmonicRelation::Adjacent,
            "Em is 9A"
        );
        assert_eq!(relation(Some("8A"), None), HarmonicRelation::Unknown);
        assert_eq!(
            relation(Some("junk"), Some("8A")),
            HarmonicRelation::Unknown
        );
    }

    #[test]
    fn bpm_delta_is_a_signed_percentage_of_the_first_tempo() {
        assert_eq!(bpm_delta_percent(Some(120.0), Some(126.0)), Some(5.0));
        assert_eq!(bpm_delta_percent(Some(125.0), Some(120.0)), Some(-4.0));
        assert_eq!(bpm_delta_percent(None, Some(120.0)), None);
        assert_eq!(bpm_delta_percent(Some(0.0), Some(120.0)), None);
    }
}
