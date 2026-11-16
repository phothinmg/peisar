#[derive(Debug, PartialEq, Clone)]
pub struct TomlDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TomlTime {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub nanosecond: u32,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TomlTzOffset {
    Z,
    Custom { minutes: i16 }, // e.g., +06:30 becomes 390, -05:00 becomes -300
}

#[derive(Debug, PartialEq, Clone)]
pub struct TomlDatetime {
    pub date: Option<TomlDate>,
    pub time: Option<TomlTime>,
    pub offset: Option<TomlTzOffset>,
}
impl TomlDatetime {
    pub fn parse(raw: &str) -> Result<Self, String> {
        // Case 1: Time-only (e.g., "07:30:00" or "07:30:00.123")
        if raw.len() >= 8 && raw.as_bytes().get(2) == Some(&b':') {
            let (time, rest) = Self::parse_time(raw)?;
            if !rest.is_empty() {
                return Err(format!("Unexpected trailing characters in time: '{}'", raw));
            }
            return Ok(Self {
                date: None,
                time: Some(time),
                offset: None,
            });
        }

        // Case 2: Starts with Date ("YYYY-MM-DD")
        if raw.len() < 10 {
            return Err(format!("Datetime too short: '{}'", raw));
        }

        let date = Self::parse_date(&raw[0..10])?;
        let rest = &raw[10..];

        // Date-only (e.g., "2026-11-17")
        if rest.is_empty() {
            return Ok(Self {
                date: Some(date),
                time: None,
                offset: None,
            });
        }

        // Must have 'T', 't', or space separator before time
        let sep = rest.as_bytes()[0];
        if sep != b'T' && sep != b't' && sep != b' ' {
            return Err(format!("Expected 'T' or space after date in '{}'", raw));
        }

        let (time, offset_str) = Self::parse_time(&rest[1..])?;
        let offset = if offset_str.is_empty() {
            None
        } else {
            Some(Self::parse_offset(offset_str)?)
        };

        Ok(Self {
            date: Some(date),
            time: Some(time),
            offset,
        })
    }
    pub fn parse_date(s: &str) -> Result<TomlDate, String> {
        if s.as_bytes()[4] != b'-' || s.as_bytes()[7] != b'-' {
            return Err(format!("Invalid date separators: '{}'", s));
        }
        let year: u16 = s[0..4].parse().map_err(|_| "Invalid year")?;
        let month: u8 = s[5..7].parse().map_err(|_| "Invalid month")?;
        let day: u8 = s[8..10].parse().map_err(|_| "Invalid day")?;

        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return Err(format!("Month or day out of range: '{}'", s));
        }

        Ok(TomlDate { year, month, day })
    }
    pub fn parse_time(s: &str) -> Result<(TomlTime, &str), String> {
        if s.len() < 8 || s.as_bytes()[2] != b':' || s.as_bytes()[5] != b':' {
            return Err(format!("Invalid time format: '{}'", s));
        }

        let hour: u8 = s[0..2].parse().map_err(|_| "Invalid hour")?;
        let minute: u8 = s[3..5].parse().map_err(|_| "Invalid minute")?;
        let second: u8 = s[6..8].parse().map_err(|_| "Invalid second")?;

        if hour > 23 || minute > 59 || second > 60 {
            return Err(format!("Time component out of range: '{}'", s));
        }

        let mut rest = &s[8..];
        let mut nanosecond = 0;

        // Parse optional fractional seconds (e.g., ".123456")
        if let Some(after_dot) = rest.strip_prefix('.') {
            let frac_len = after_dot.bytes().take_while(|b| b.is_ascii_digit()).count();
            if frac_len == 0 {
                return Err("Expected digits after decimal point in time".into());
            }
            let frac_str = &after_dot[..frac_len.min(9)];
            let mut val: u32 = frac_str.parse().map_err(|_| "Invalid fractional second")?;
            // Scale up to 9 digits (nanoseconds)
            for _ in frac_str.len()..9 {
                val *= 10;
            }
            nanosecond = val;
            rest = &after_dot[frac_len..];
        }

        Ok((
            TomlTime {
                hour,
                minute,
                second,
                nanosecond,
            },
            rest,
        ))
    }
    pub fn parse_offset(s: &str) -> Result<TomlTzOffset, String> {
        if s == "Z" || s == "z" {
            return Ok(TomlTzOffset::Z);
        }

        if s.len() != 6 || s.as_bytes()[3] != b':' {
            return Err(format!("Invalid timezone offset: '{}'", s));
        }

        let sign: i16 = match s.as_bytes()[0] {
            b'+' => 1,
            b'-' => -1,
            _ => return Err(format!("Invalid offset sign in '{}'", s)),
        };

        let hours: i16 = s[1..3].parse().map_err(|_| "Invalid offset hours")?;
        let mins: i16 = s[4..6].parse().map_err(|_| "Invalid offset minutes")?;

        if hours > 23 || mins > 59 {
            return Err(format!("Offset out of range: '{}'", s));
        }

        Ok(TomlTzOffset::Custom {
            minutes: sign * (hours * 60 + mins),
        })
    }
    pub fn to_iso_string(&self) -> String {
        let mut out = String::new();
        if let Some(d) = &self.date {
            out.push_str(&format!("{:04}-{:02}-{:02}", d.year, d.month, d.day));
        }
        if self.date.is_some() && self.time.is_some() {
            out.push('T');
        }
        if let Some(t) = &self.time {
            out.push_str(&format!("{:02}:{:02}:{:02}", t.hour, t.minute, t.second));
            if t.nanosecond > 0 {
                let frac = format!("{:09}", t.nanosecond);
                out.push('.');
                out.push_str(frac.trim_end_matches('0'));
            }
        }
        if let Some(off) = &self.offset {
            match off {
                TomlTzOffset::Z => out.push('Z'),
                TomlTzOffset::Custom { minutes } => {
                    let sign = if *minutes >= 0 { '+' } else { '-' };
                    let abs = minutes.abs();
                    out.push_str(&format!("{}{:02}:{:02}", sign, abs / 60, abs % 60));
                }
            }
        }
        out
    }
}
