//! Tiny parser for the YAML subset iRacing emits in its session info string.
//!
//! iRacing's YAML is not always valid YAML (driver and team names are written
//! unquoted and may contain `:`, `#`, quotes, ...), so instead of a general YAML
//! library we parse its simple indentation-based structure directly: every
//! value runs to the end of the line.

#[derive(Debug, Clone, PartialEq)]
pub enum Yaml {
    Null,
    Str(String),
    List(Vec<Yaml>),
    Map(Vec<(String, Yaml)>),
}

static NULL: Yaml = Yaml::Null;

impl Yaml {
    pub fn get(&self, key: &str) -> &Yaml {
        match self {
            Yaml::Map(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v).unwrap_or(&NULL),
            _ => &NULL,
        }
    }

    /// Follow a dotted path, e.g. `"WeekendInfo.WeekendOptions.IncidentLimit"`.
    pub fn path(&self, path: &str) -> &Yaml {
        path.split('.').fold(self, |n, k| n.get(k))
    }

    pub fn list(&self) -> &[Yaml] {
        match self {
            Yaml::List(l) => l,
            _ => &[],
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Yaml::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn string(&self) -> String {
        self.str().unwrap_or_default().to_string()
    }

    /// Leading number of a scalar: handles `"5.79 km"`, `"33.20 C"`, `"0xffda59"`.
    pub fn f64(&self) -> Option<f64> {
        let s = self.str()?.trim();
        if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            return i64::from_str_radix(hex.split_whitespace().next()?, 16).ok().map(|v| v as f64);
        }
        let end = s
            .char_indices()
            .find(|&(i, c)| !(c.is_ascii_digit() || c == '.' || ((c == '-' || c == '+') && i == 0) || c == 'e' || c == 'E'))
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        s[..end].parse().ok()
    }

    pub fn i64(&self) -> Option<i64> {
        self.f64().map(|v| v as i64)
    }

    pub fn i32_or(&self, d: i32) -> i32 {
        self.i64().map(|v| v as i32).unwrap_or(d)
    }

    pub fn f32_or(&self, d: f32) -> f32 {
        self.f64().map(|v| v as f32).unwrap_or(d)
    }
}

struct Line<'a> {
    indent: usize,
    text: &'a str,
}

pub fn parse(src: &str) -> Yaml {
    let lines: Vec<Line> = src
        .lines()
        .filter_map(|l| {
            let l = l.trim_end_matches('\r');
            let t = l.trim_start_matches(' ');
            if t.is_empty() || t == "---" || t == "..." {
                return None;
            }
            Some(Line { indent: l.len() - t.len(), text: t })
        })
        .collect();
    let mut p = Parser { lines, pos: 0, pending: None };
    if p.lines.is_empty() {
        return Yaml::Map(vec![]);
    }
    let indent = p.lines[0].indent;
    p.block(indent)
}

struct Parser<'a> {
    lines: Vec<Line<'a>>,
    pos: usize,
    /// Remainder of a `- key: value` line, treated as a virtual line at `.0` indent.
    pending: Option<(usize, &'a str)>,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<(usize, &'a str)> {
        self.pending.or_else(|| self.lines.get(self.pos).map(|l| (l.indent, l.text)))
    }

    fn advance(&mut self) {
        if self.pending.take().is_none() {
            self.pos += 1;
        }
    }

    fn block(&mut self, indent: usize) -> Yaml {
        match self.peek() {
            Some((_, t)) if is_item(t) => self.list(indent),
            Some(_) => self.map(indent),
            None => Yaml::Null,
        }
    }

    fn list(&mut self, indent: usize) -> Yaml {
        let mut items = Vec::new();
        while let Some((ind, t)) = self.peek() {
            if ind != indent || !is_item(t) {
                break;
            }
            let rest = t[1..].trim_start_matches(' ');
            let child_indent = indent + (t.len() - rest.len());
            self.advance();
            if rest.is_empty() {
                match self.peek() {
                    Some((i, _)) if i > indent => {
                        let item = self.block(i);
                        items.push(item);
                    }
                    _ => items.push(Yaml::Null),
                }
            } else if split_kv(rest).is_some() {
                self.pending = Some((child_indent, rest));
                items.push(self.map(child_indent));
            } else {
                items.push(scalar(rest));
            }
        }
        Yaml::List(items)
    }

    fn map(&mut self, indent: usize) -> Yaml {
        let mut entries = Vec::new();
        while let Some((ind, t)) = self.peek() {
            if ind != indent || is_item(t) {
                break;
            }
            self.advance();
            let Some((k, v)) = split_kv(t) else { continue };
            let value = if !v.is_empty() {
                scalar(v)
            } else {
                match self.peek() {
                    // iRacing writes list items at the same indent as their parent key.
                    Some((i, nt)) if i >= indent && is_item(nt) => self.list(i),
                    Some((i, _)) if i > indent => self.block(i),
                    _ => Yaml::Null,
                }
            };
            entries.push((k.to_string(), value));
        }
        Yaml::Map(entries)
    }
}

fn is_item(t: &str) -> bool {
    t == "-" || t.starts_with("- ")
}

fn split_kv(t: &str) -> Option<(&str, &str)> {
    if let Some(k) = t.strip_suffix(':') {
        if !k.contains(": ") {
            return Some((k.trim(), ""));
        }
    }
    let i = t.find(": ")?;
    Some((t[..i].trim(), t[i + 2..].trim()))
}

fn scalar(v: &str) -> Yaml {
    let v = v.trim();
    let unq = if v.len() >= 2 && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\''))) { &v[1..v.len() - 1] } else { v };
    Yaml::Str(unq.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "---
WeekendInfo:
 TrackName: spa 2022 up
 TrackID: 524
 TrackLength: 6.93 km
 TrackSurfaceTemp: 33.20 C
 WeekendOptions:
  IncidentLimit: 17
SessionInfo:
 Sessions:
 - SessionNum: 0
   SessionLaps: unlimited
   SessionTime: 1800.0000 sec
   SessionType: Race
   ResultsPositions:
   - Position: 1
     CarIdx: 4
   - Position: 2
     CarIdx: 1
 - SessionNum: 1
   SessionType: Practice
   ResultsPositions:
DriverInfo:
 DriverCarIdx: 1
 Drivers:
 - CarIdx: 0
   UserName: Pace Car
   CarClassColor: 0xffffff
 - CarIdx: 1
   UserName: Joe \"The: Rocket\" #1
   TeamName: Team: Speed
   CarNumber: \"07\"
SplitTimeInfo:
 Sectors:
 - SectorNum: 0
   SectorStartPct: 0.000000
 - SectorNum: 1
   SectorStartPct: 0.333
...
";

    #[test]
    fn parses_iracing_session_info() {
        let y = parse(SAMPLE);
        assert_eq!(y.path("WeekendInfo.TrackName").str(), Some("spa 2022 up"));
        assert_eq!(y.path("WeekendInfo.TrackID").i64(), Some(524));
        assert_eq!(y.path("WeekendInfo.TrackLength").f64(), Some(6.93));
        assert_eq!(y.path("WeekendInfo.TrackSurfaceTemp").f64(), Some(33.2));
        assert_eq!(y.path("WeekendInfo.WeekendOptions.IncidentLimit").i64(), Some(17));
        let sessions = y.path("SessionInfo.Sessions").list();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].get("SessionLaps").str(), Some("unlimited"));
        assert_eq!(sessions[0].get("SessionTime").f64(), Some(1800.0));
        let res = sessions[0].get("ResultsPositions").list();
        assert_eq!(res.len(), 2);
        assert_eq!(res[1].get("CarIdx").i64(), Some(1));
        assert_eq!(sessions[1].get("ResultsPositions"), &Yaml::Null);
        let drivers = y.path("DriverInfo.Drivers").list();
        assert_eq!(drivers.len(), 2);
        assert_eq!(drivers[0].get("CarClassColor").i64(), Some(0xffffff));
        assert_eq!(drivers[1].get("UserName").str(), Some("Joe \"The: Rocket\" #1"));
        assert_eq!(drivers[1].get("TeamName").str(), Some("Team: Speed"));
        assert_eq!(drivers[1].get("CarNumber").str(), Some("07"));
        let sectors = y.path("SplitTimeInfo.Sectors").list();
        assert_eq!(sectors[1].get("SectorStartPct").f64(), Some(0.333));
    }
}
