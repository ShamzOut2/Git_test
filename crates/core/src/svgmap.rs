//! Track outline from iRacing's official track map (the SVG layers iRacing shows on its
//! website): the racing line from the "active" layer, the start/finish line from the
//! "start-finish" layer and the turn numbers from the "turns" layer to tell the direction of
//! travel. The line is resampled by distance so that point i sits at lap fraction i / BINS,
//! like a recorded outline.

use crate::engine::trackmap::{normalise, TrackMap, BINS};

type P = [f64; 2];
type M = [f64; 6];

const ID: M = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn mul(a: &M, b: &M) -> M {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

fn apply(m: &M, p: P) -> P {
    [m[0] * p[0] + m[2] * p[1] + m[4], m[1] * p[0] + m[3] * p[1] + m[5]]
}

fn numbers(s: &str) -> Vec<f64> {
    let mut out = vec![];
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c.is_ascii_digit() || c == '-' || c == '+' || c == '.' {
            let start = i;
            i += 1;
            let mut dot = c == '.';
            while i < b.len() {
                let d = b[i] as char;
                if d.is_ascii_digit() {
                    i += 1;
                } else if d == '.' && !dot {
                    dot = true;
                    i += 1;
                } else if (d == 'e' || d == 'E') && i + 1 < b.len() {
                    i += 1;
                    if b[i] == b'-' || b[i] == b'+' {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            if let Ok(v) = s[start..i].parse::<f64>() {
                out.push(v);
            }
        } else {
            i += 1;
        }
    }
    out
}

/// `transform="translate(..) matrix(..) scale(..) rotate(..)"`.
fn transform(s: &str) -> M {
    let mut m = ID;
    for part in s.split(')') {
        let Some((name, args)) = part.split_once('(') else { continue };
        let v = numbers(args);
        let t = match name.trim().trim_start_matches(',').trim() {
            "matrix" if v.len() == 6 => [v[0], v[1], v[2], v[3], v[4], v[5]],
            "translate" => [1.0, 0.0, 0.0, 1.0, v.first().copied().unwrap_or(0.0), v.get(1).copied().unwrap_or(0.0)],
            "scale" => {
                let sx = v.first().copied().unwrap_or(1.0);
                [sx, 0.0, 0.0, v.get(1).copied().unwrap_or(sx), 0.0, 0.0]
            }
            "rotate" => {
                let a = v.first().copied().unwrap_or(0.0).to_radians();
                let r = [a.cos(), a.sin(), -a.sin(), a.cos(), 0.0, 0.0];
                match (v.get(1), v.get(2)) {
                    (Some(&cx), Some(&cy)) => mul(&mul(&[1.0, 0.0, 0.0, 1.0, cx, cy], &r), &[1.0, 0.0, 0.0, 1.0, -cx, -cy]),
                    _ => r,
                }
            }
            _ => ID,
        };
        m = mul(&m, &t);
    }
    m
}

/// Subpaths of an SVG path, curves flattened.
fn path(d: &str) -> Vec<Vec<P>> {
    let mut subs: Vec<Vec<P>> = vec![];
    let mut cur: Vec<P> = vec![];
    let (mut pos, mut start, mut ctrl): (P, P, Option<P>) = ([0.0; 2], [0.0; 2], None);
    // Tokens: command letters and numbers.
    let mut toks: Vec<(char, Vec<f64>)> = vec![];
    let mut cmd = ' ';
    let mut buf = String::new();
    for c in d.chars() {
        if c.is_ascii_alphabetic() && c != 'e' && c != 'E' {
            if cmd != ' ' {
                toks.push((cmd, numbers(&buf)));
            }
            cmd = c;
            buf.clear();
        } else {
            buf.push(c);
        }
    }
    if cmd != ' ' {
        toks.push((cmd, numbers(&buf)));
    }
    let flush = |cur: &mut Vec<P>, subs: &mut Vec<Vec<P>>| {
        if cur.len() > 1 {
            subs.push(std::mem::take(cur));
        } else {
            cur.clear();
        }
    };
    let cubic = |cur: &mut Vec<P>, p0: P, c1: P, c2: P, p: P| {
        for k in 1..=12 {
            let t = k as f64 / 12.0;
            let u = 1.0 - t;
            cur.push([
                u * u * u * p0[0] + 3.0 * u * u * t * c1[0] + 3.0 * u * t * t * c2[0] + t * t * t * p[0],
                u * u * u * p0[1] + 3.0 * u * u * t * c1[1] + 3.0 * u * t * t * c2[1] + t * t * t * p[1],
            ]);
        }
    };
    for (c, v) in toks {
        let rel = c.is_ascii_lowercase();
        let off = |p: P, pos: P| if rel { [p[0] + pos[0], p[1] + pos[1]] } else { p };
        match c.to_ascii_uppercase() {
            'M' => {
                flush(&mut cur, &mut subs);
                for (i, xy) in v.as_chunks::<2>().0.iter().enumerate() {
                    pos = off([xy[0], xy[1]], pos);
                    if i == 0 {
                        start = pos;
                    }
                    cur.push(pos);
                }
                ctrl = None;
            }
            'L' | 'T' => {
                for xy in v.as_chunks::<2>().0.iter() {
                    pos = off([xy[0], xy[1]], pos);
                    cur.push(pos);
                }
                ctrl = None;
            }
            'H' => {
                for x in &v {
                    pos = [if rel { pos[0] + x } else { *x }, pos[1]];
                    cur.push(pos);
                }
                ctrl = None;
            }
            'V' => {
                for y in &v {
                    pos = [pos[0], if rel { pos[1] + y } else { *y }];
                    cur.push(pos);
                }
                ctrl = None;
            }
            'C' => {
                for w in v.as_chunks::<6>().0.iter() {
                    let (c1, c2, p) = (off([w[0], w[1]], pos), off([w[2], w[3]], pos), off([w[4], w[5]], pos));
                    cubic(&mut cur, pos, c1, c2, p);
                    ctrl = Some(c2);
                    pos = p;
                }
            }
            'S' => {
                for w in v.as_chunks::<4>().0.iter() {
                    let c1 = ctrl.map(|c| [2.0 * pos[0] - c[0], 2.0 * pos[1] - c[1]]).unwrap_or(pos);
                    let (c2, p) = (off([w[0], w[1]], pos), off([w[2], w[3]], pos));
                    cubic(&mut cur, pos, c1, c2, p);
                    ctrl = Some(c2);
                    pos = p;
                }
            }
            'Q' => {
                for w in v.as_chunks::<4>().0.iter() {
                    let (q, p) = (off([w[0], w[1]], pos), off([w[2], w[3]], pos));
                    let c1 = [pos[0] + 2.0 / 3.0 * (q[0] - pos[0]), pos[1] + 2.0 / 3.0 * (q[1] - pos[1])];
                    let c2 = [p[0] + 2.0 / 3.0 * (q[0] - p[0]), p[1] + 2.0 / 3.0 * (q[1] - p[1])];
                    cubic(&mut cur, pos, c1, c2, p);
                    ctrl = None;
                    pos = p;
                }
            }
            'A' => {
                // Arcs are rare in track maps: a straight line to the end point is close enough.
                for w in v.as_chunks::<7>().0.iter() {
                    pos = off([w[5], w[6]], pos);
                    cur.push(pos);
                }
                ctrl = None;
            }
            'Z' => {
                if cur.first() != Some(&pos) {
                    cur.push(start);
                }
                pos = start;
                flush(&mut cur, &mut subs);
                ctrl = None;
            }
            _ => {}
        }
    }
    flush(&mut cur, &mut subs);
    subs
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = tag;
    while let Some(i) = rest.find(name) {
        let before = rest[..i].chars().last();
        let after = rest[i + name.len()..].trim_start();
        if before.is_some_and(|c| c.is_whitespace()) && after.starts_with('=') {
            let v = after[1..].trim_start();
            let q = v.chars().next()?;
            if q == '"' || q == '\'' {
                let end = v[1..].find(q)?;
                return Some(&v[1..1 + end]);
            }
        }
        rest = &rest[i + name.len()..];
    }
    None
}

#[derive(Default)]
struct Svg {
    /// Polylines of every shape, in document coordinates.
    shapes: Vec<Vec<P>>,
    /// Text labels and where they are.
    texts: Vec<(String, P)>,
}

fn parse(svg: &str) -> Svg {
    let mut out = Svg::default();
    let mut stack: Vec<M> = vec![ID];
    let mut i = 0;
    while let Some(o) = svg[i..].find('<') {
        let s = i + o;
        let Some(e) = svg[s..].find('>') else { break };
        let tag = &svg[s + 1..s + e];
        i = s + e + 1;
        if let Some(close) = tag.strip_prefix('/') {
            if close.trim_start().starts_with("g") && stack.len() > 1 {
                stack.pop();
            }
            continue;
        }
        let name: String = tag.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        let selfclose = tag.ends_with('/');
        let m = mul(stack.last().unwrap(), &attr(tag, "transform").map(transform).unwrap_or(ID));
        let f = |k: &str| attr(tag, k).and_then(|v| numbers(v).first().copied());
        let tr = |pts: Vec<P>| pts.into_iter().map(|p| apply(&m, p)).collect::<Vec<_>>();
        match name.as_str() {
            "g" if !selfclose => stack.push(m),
            "path" => {
                for sub in path(attr(tag, "d").unwrap_or("")) {
                    out.shapes.push(tr(sub));
                }
            }
            "polygon" | "polyline" => {
                let mut pts: Vec<P> = numbers(attr(tag, "points").unwrap_or("")).as_chunks::<2>().0.to_vec();
                if name == "polygon" && pts.len() > 2 {
                    pts.push(pts[0]);
                }
                out.shapes.push(tr(pts));
            }
            "line" => out.shapes.push(tr(vec![[f("x1").unwrap_or(0.0), f("y1").unwrap_or(0.0)], [f("x2").unwrap_or(0.0), f("y2").unwrap_or(0.0)]])),
            "rect" => {
                let (x, y, w, h) = (f("x").unwrap_or(0.0), f("y").unwrap_or(0.0), f("width").unwrap_or(0.0), f("height").unwrap_or(0.0));
                out.shapes.push(tr(vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h], [x, y]]));
            }
            "circle" | "ellipse" => out.shapes.push(tr(vec![[f("cx").unwrap_or(0.0), f("cy").unwrap_or(0.0)]])),
            "text" if !selfclose => {
                // Text content up to </text>, without nested tags (tspans).
                let end = svg[i..].find("</text").map(|k| i + k).unwrap_or(i);
                let mut content = String::new();
                let mut inside = false;
                for c in svg[i..end].chars() {
                    match c {
                        '<' => inside = true,
                        '>' => inside = false,
                        c if !inside => content.push(c),
                        _ => {}
                    }
                }
                // Position: x/y on the text or its first tspan, plus the transform.
                let pos = [f("x").unwrap_or(0.0), f("y").unwrap_or(0.0)];
                let pos = if pos == [0.0, 0.0] {
                    let inner = &svg[i..end];
                    inner
                        .find("<tspan")
                        .map(|k| {
                            let t = &inner[k..inner[k..].find('>').map(|e| k + e).unwrap_or(inner.len())];
                            let g = |n: &str| attr(t, n).and_then(|v| numbers(v).first().copied()).unwrap_or(0.0);
                            [g("x"), g("y")]
                        })
                        .unwrap_or(pos)
                } else {
                    pos
                };
                out.texts.push((content.trim().to_string(), apply(&m, pos)));
                i = end;
            }
            _ => {}
        }
    }
    out
}

fn length(pts: &[P]) -> f64 {
    pts.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum()
}

/// Arc position (0..len) of the point on the closed line nearest to `p`.
fn project(line: &[P], cum: &[f64], p: P) -> f64 {
    let mut best = (f64::MAX, 0.0);
    for (k, w) in line.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l2 = dx * dx + dy * dy;
        let t = if l2 > 0.0 { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
        let q = [a[0] + dx * t, a[1] + dy * t];
        let d = (p[0] - q[0]).hypot(p[1] - q[1]);
        if d < best.0 {
            best = (d, cum[k] + t * l2.sqrt());
        }
    }
    best.1
}

fn at(line: &[P], cum: &[f64], s: f64) -> P {
    let total = *cum.last().unwrap();
    let s = s.rem_euclid(total);
    let k = cum.partition_point(|&c| c <= s).clamp(1, line.len() - 1);
    let (a, b) = (line[k - 1], line[k]);
    let seg = cum[k] - cum[k - 1];
    let t = if seg > 0.0 { (s - cum[k - 1]) / seg } else { 0.0 };
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn centre(shapes: &[Vec<P>]) -> Option<P> {
    let pts: Vec<&P> = shapes.iter().flatten().collect();
    if pts.is_empty() {
        return None;
    }
    let n = pts.len() as f64;
    Some([pts.iter().map(|p| p[0]).sum::<f64>() / n, pts.iter().map(|p| p[1]).sum::<f64>() / n])
}

/// Build the outline from the SVG layers. `t1_hint`: lap fraction of turn 1 when known (e.g.
/// from Lovely Track Data), the most reliable way to tell the direction of travel.
pub fn build(track_id: i64, name: &str, active: &str, start_finish: Option<&str>, turns: Option<&str>, t1_hint: Option<f32>) -> Option<TrackMap> {
    let mut line = parse(active).shapes.into_iter().max_by(|a, b| length(a).total_cmp(&length(b)))?;
    if line.len() < 4 {
        return None;
    }
    if line.first() != line.last() {
        line.push(line[0]);
    }
    let mut cum = vec![0.0];
    for w in line.windows(2) {
        cum.push(cum.last().unwrap() + (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]));
    }
    let total = *cum.last().unwrap();
    if total <= 0.0 {
        return None;
    }
    let s0 = start_finish.and_then(|s| centre(&parse(s).shapes)).map(|p| project(&line, &cum, p)).unwrap_or(0.0);
    // Turn labels: the lowest number is turn 1.
    let mut labels: Vec<(u32, P)> = turns
        .map(|t| {
            parse(t)
                .texts
                .into_iter()
                .filter_map(|(txt, p)| txt.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().ok().map(|n| (n, p)))
                .collect()
        })
        .unwrap_or_default();
    labels.sort_by_key(|l| l.0);
    let ahead = |p: P| (project(&line, &cum, p) - s0).rem_euclid(total) / total;
    let forward = match (t1_hint, labels.first(), labels.get(1)) {
        (Some(h), Some(t1), _) => {
            let f = ahead(t1.1);
            ((f - h as f64).abs()) <= ((1.0 - f) - h as f64).abs()
        }
        (_, Some(t1), Some(t2)) => ahead(t1.1) < ahead(t2.1),
        (_, Some(t1), None) => ahead(t1.1) < 0.5,
        _ => true,
    };
    let dir = if forward { 1.0 } else { -1.0 };
    // SVG y points down; `normalise` expects y up.
    let pts: Vec<[f64; 2]> = (0..BINS)
        .map(|i| {
            let p = at(&line, &cum, s0 + dir * total * i as f64 / BINS as f64);
            [p[0], -p[1]]
        })
        .collect();
    Some(TrackMap { track_id, name: name.to_string(), points: normalise(&pts), source: "iracing".into() })
}

/// Where the published index of iRacing's track maps lives (track id → map folder and
/// layer files), so maps work without an iRacing login. Maintainers refresh it from the
/// `iracing-track-maps.json` a logged-in copy of the app writes (see `trim_index`).
const INDEX_URL: &str = "https://raw.githubusercontent.com/ShamzOut2/Apex_Overlays/main/data/iracing-track-maps.json";

/// The part of the Data API's track/assets that locates each track map: the SVG files
/// themselves are public, only this index needs a login.
pub fn trim_index(assets: &serde_json::Value) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    for (id, e) in assets.as_object().into_iter().flatten() {
        let (Some(base), Some(layers)) = (e.get("track_map").and_then(|b| b.as_str()), e.get("track_map_layers")) else { continue };
        if base.is_empty() || !layers.is_object() {
            continue;
        }
        out.insert(id.clone(), serde_json::json!({ "track_map": base, "track_map_layers": layers }));
    }
    serde_json::Value::Object(out)
}

/// Downloads iRacing's track map for the current track and caches the outline per track.
/// The index of maps comes from the Data API when the iRacing login is set up (League
/// builder → Import from iRacing), otherwise from the copy published with Apex Overlays.
pub fn spawn(
    hub: std::sync::Arc<crate::hub::Hub>,
    config: crate::config::ConfigStore,
    data_dir: std::path::PathBuf,
    track: std::sync::Arc<std::sync::Mutex<(i64, String)>>,
) {
    use crate::hub::Command;
    use serde_json::Value;
    use std::time::{Duration, Instant, SystemTime};
    let _ = std::thread::Builder::new().name("trackmaps".into()).spawn(move || {
        let dir = data_dir.join("trackmaps").join("iracing");
        let assets_file = data_dir.join("iracing").join("track-assets.json");
        let index_file = data_dir.join("iracing").join("published-track-maps.json");
        let agent = crate::net::agent();
        let mut client: Option<crate::iracing_api::Client> = None;
        let mut done: Option<i64> = None;
        let mut retry_at = Instant::now();
        let read = |p: &std::path::Path| std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok());
        loop {
            std::thread::sleep(Duration::from_secs(3));
            let (id, name) = track.lock().unwrap().clone();
            if id == 0 || done == Some(id) {
                continue;
            }
            let cache = dir.join(format!("{id}.json"));
            if let Some(m) = read(&cache).and_then(|v| serde_json::from_value::<TrackMap>(v).ok()) {
                hub.command(Command::SetOfficialMap(m));
                done = Some(id);
                continue;
            }
            if Instant::now() < retry_at {
                continue;
            }
            let api = config.read(|c| c.iracing_api.clone());
            let key = id.to_string();
            // Track map index (all tracks): cached for a week, refreshed when the track is missing.
            let fresh = |p: &std::path::Path| {
                std::fs::metadata(p)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| SystemTime::now().duration_since(t).ok())
                    .is_some_and(|age| age < Duration::from_secs(7 * 86_400))
            };
            let file = if api.configured() { &assets_file } else { &index_file };
            let mut assets = read(file).filter(|a| fresh(file) && a.get(&key).is_some());
            if assets.is_none() && api.configured() {
                let c = client.get_or_insert_with(|| crate::iracing_api::Client::new(api.clone()));
                match c.get("track/assets", &[]) {
                    Ok(v) => {
                        if let Some(d) = assets_file.parent() {
                            let _ = std::fs::create_dir_all(d);
                        }
                        let _ = std::fs::write(&assets_file, v.to_string());
                        // The trimmed index, ready to publish for users without a login.
                        let _ = std::fs::write(
                            assets_file.with_file_name("iracing-track-maps.json"),
                            serde_json::to_string_pretty(&trim_index(&v)).unwrap_or_default(),
                        );
                        assets = Some(v);
                    }
                    Err(e) => {
                        log::warn!("track maps: {e}");
                        client = None;
                        retry_at = Instant::now() + Duration::from_secs(600);
                        continue;
                    }
                }
            } else if assets.is_none() {
                match agent.get(INDEX_URL).call().ok().and_then(|r| r.into_json::<Value>().ok()) {
                    Some(v) => {
                        if let Some(d) = index_file.parent() {
                            let _ = std::fs::create_dir_all(d);
                        }
                        let _ = std::fs::write(&index_file, v.to_string());
                        if v.get(&key).is_none() {
                            // Not in the published index (yet): the recorded outline is used.
                            log::info!("track {id} is not in the published track map index");
                            done = Some(id);
                            continue;
                        }
                        assets = Some(v);
                    }
                    None => {
                        retry_at = Instant::now() + Duration::from_secs(3600);
                        continue;
                    }
                }
            }
            let published = assets_file.with_file_name("iracing-track-maps.json");
            if api.configured() && !published.exists() {
                if let Some(a) = &assets {
                    let _ = std::fs::write(&published, serde_json::to_string_pretty(&trim_index(a)).unwrap_or_default());
                }
            }
            let entry = assets.as_ref().and_then(|a| a.get(&key));
            let base = entry.and_then(|e| e.get("track_map")).and_then(|b| b.as_str()).unwrap_or("");
            let layers = entry.and_then(|e| e.get("track_map_layers"));
            let layer = |k: &str| -> Option<String> {
                let file = layers?.get(k)?.as_str()?;
                let url = if file.starts_with("http") { file.to_string() } else { format!("{}/{}", base.trim_end_matches('/'), file.trim_start_matches('/')) };
                agent.get(&url).call().ok()?.into_string().ok()
            };
            let Some(active) = layer("active") else {
                log::info!("no iRacing track map for track {id}");
                done = Some(id);
                continue;
            };
            // Turn 1 from Lovely Track Data (cached by the corners downloader), if known.
            let hint = read(&data_dir.join("corners").join(format!("{id}.v2.json")))
                .and_then(|v| serde_json::from_value::<crate::corners::TrackData>(v).ok())
                .and_then(|d| d.corners.iter().find(|c| c.number == 1).map(|c| c.start));
            match build(id, &name, &active, layer("start-finish").as_deref(), layer("turns").as_deref(), hint) {
                Some(m) => {
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = std::fs::write(&cache, serde_json::to_string(&m).unwrap_or_default());
                    hub.command(Command::SetOfficialMap(m));
                }
                None => log::warn!("could not read the iRacing track map for track {id}"),
            }
            done = Some(id);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // A 200 x 100 rectangle; start/finish halfway along the top; turn 1 at the top-right corner,
    // so cars run clockwise on screen (left to right along the top).
    const ACTIVE: &str = r#"<svg viewBox="0 0 300 200"><g transform="translate(50 50)"><path d="M0,0 H200 V100 h-200 Z" fill="none"/></g></svg>"#;
    const SF: &str = r#"<svg><g transform="translate(50 50)"><line x1="100" y1="-5" x2="100" y2="5"/></g></svg>"#;
    const TURNS: &str =
        r#"<svg><text transform="matrix(1 0 0 1 255 45)">1</text><text x="255" y="155">2</text><text><tspan x="45" y="155">3</tspan></text></svg>"#;

    fn screen(m: &TrackMap, i: usize) -> [f32; 2] {
        m.points[i]
    }

    #[test]
    fn trims_the_asset_index() {
        let v = serde_json::json!({
            "1": {"track_id": 1, "track_map": "https://x/tracks/a/1/", "track_map_layers": {"active": "active.svg", "turns": "turns.svg"}, "detail_copy": "long text"},
            "2": {"track_id": 2, "track_map": "", "track_map_layers": {}},
            "3": {"track_id": 3}
        });
        let t = trim_index(&v);
        assert_eq!(t, serde_json::json!({"1": {"track_map": "https://x/tracks/a/1/", "track_map_layers": {"active": "active.svg", "turns": "turns.svg"}}}));
    }

    #[test]
    fn parses_shapes_and_text() {
        let s = parse(TURNS);
        assert_eq!(s.texts.len(), 3);
        assert_eq!(s.texts[0], ("1".to_string(), [255.0, 45.0]));
        assert_eq!(s.texts[2].1, [45.0, 155.0]);
        let a = parse(ACTIVE);
        assert_eq!(a.shapes.len(), 1);
        assert_eq!(a.shapes[0][1], [250.0, 50.0]);
        assert_eq!(path("M0 0c10 0 10 10 10 10s0 10-10 10z")[0].last(), Some(&[0.0, 0.0]));
    }

    #[test]
    fn builds_outline_from_layers() {
        let m = build(1, "Test", ACTIVE, Some(SF), Some(TURNS), None).unwrap();
        assert_eq!(m.points.len(), BINS);
        // Starts at the start/finish line (top middle) and heads right.
        let (p0, p1) = (screen(&m, 0), screen(&m, 20));
        assert!((p0[0] - 0.5).abs() < 0.01, "{p0:?}");
        assert!(p1[0] > p0[0] + 0.05 && (p1[1] - p0[1]).abs() < 0.01, "{p0:?} {p1:?}");
        // A quarter of the lap (150 of 600) in: past turn 1, on the right-hand side.
        let q = screen(&m, BINS / 4);
        assert!(q[0] > 0.95, "{q:?}");
        // Turns numbered the other way round (turn 1 bottom left): the map runs anticlockwise.
        let rev = r#"<svg><text x="45" y="155">1</text><text x="255" y="155">2</text><text x="255" y="45">3</text></svg>"#;
        let r = build(1, "Test", ACTIVE, Some(SF), Some(rev), None).unwrap();
        assert!(screen(&r, 20)[0] < screen(&r, 0)[0]);
        // A Lovely hint wins: turn 1 two thirds round the lap only fits clockwise.
        let h = build(1, "Test", ACTIVE, Some(SF), Some(rev), Some(0.66)).unwrap();
        assert!(screen(&h, 20)[0] > screen(&h, 0)[0]);
    }
}
