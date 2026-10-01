//! Identifier facts read from a map's scripts, for `--campaign-graph-check` only (verification,
//! never gameplay; `docs/CAMPAIGN_PLAN.md` section 3.3). The scanner never interprets a script.
//! It keeps three kinds of identifiers and drops everything else:
//!
//! * the verbs a script applies to a named entity: it enables it (`E`), disables it (`D`), fires
//!   it (`F`, through the trigger call) or binds it to a mover (`B`);
//! * the destination and the enclosing function of every level change the script requests;
//! * the names of the functions the scripts define.
//!
//! Text inside comments is invisible to it, so a block that the authors commented out (the hedge
//! maze's dead level change) is not a fact.
use crate::assets::Assets;
use anyhow::{ensure, Result};
use std::collections::{BTreeMap, BTreeSet};

/// One level change a script requests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapCall {
    /// The destination as the script spells it (`map$entrance` or a bare map).
    pub dest: String,
    /// The function the call sits in.
    pub function: String,
}

#[derive(Default, Debug)]
pub struct Scan {
    verbs: BTreeMap<String, BTreeSet<char>>,
    pub calls: Vec<MapCall>,
    pub functions: BTreeSet<String>,
    pub files: usize,
}

impl Scan {
    /// The verbs applied to `target`, as sorted letters (`"BDE"`), empty when none.
    pub fn verbs(&self, target: &str) -> String {
        self.verbs
            .get(target)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Read every script of a map: its own, its cinematics, and whatever they include.
    pub fn of_map(assets: &mut Assets, map: &str) -> Result<Self> {
        let mut pending = vec![format!("maps/{map}.scr")];
        let prefix = format!("maps/cinematics/{map}");
        let mut extra: Vec<String> = assets
            .names()
            .filter(|n| n.starts_with(&prefix) && n.ends_with(".scr"))
            .map(str::to_owned)
            .collect();
        extra.sort();
        pending.extend(extra);
        let mut scan = Self::default();
        let mut seen = BTreeSet::new();
        while let Some(path) = pending.pop() {
            let path = path.replace('\\', "/").to_ascii_lowercase();
            if !seen.insert(path.clone()) || !assets.contains(&path) {
                continue;
            }
            ensure!(seen.len() <= 128, "{map}: too many script includes");
            let text = String::from_utf8_lossy(&assets.read(&path)?).into_owned();
            let clean = strip_comments(&text);
            for include in includes(&clean) {
                let key = include
                    .replace('\\', "/")
                    .replace("../", "")
                    .to_ascii_lowercase();
                for candidate in [key.clone(), format!("maps/{key}"), format!("global/{key}")] {
                    if assets.contains(&candidate) {
                        pending.push(candidate);
                        break;
                    }
                }
            }
            scan.add(&clean);
            scan.files += 1;
        }
        Ok(scan)
    }

    /// Fold one comment-free script into the facts.
    pub fn add(&mut self, clean: &str) {
        let mut current = String::new();
        for line in clean.lines() {
            if let Some(name) = function_definition(line) {
                self.functions.insert(name.to_owned());
                current = name.to_owned();
            }
            for dest in map_calls(line) {
                self.calls.push(MapCall {
                    dest,
                    function: current.clone(),
                });
            }
            for (target, verb) in verbs(line) {
                self.verbs.entry(target).or_default().insert(verb);
            }
        }
    }
}

/// Remove `//` and `/* */` comments, keeping newlines and quoted text intact.
pub fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let (mut block, mut line, mut quote) = (false, false, false);
    while let Some(c) = chars.next() {
        if c == '\n' || c == '\r' {
            out.push('\n');
            line = false;
            continue;
        }
        if block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                block = false;
                out.push(' ');
            }
            continue;
        }
        if line {
            continue;
        }
        if !quote && c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            block = true;
            continue;
        }
        if !quote && c == '/' && chars.peek() == Some(&'/') {
            chars.next();
            line = true;
            continue;
        }
        if c == '"' {
            quote = !quote;
        }
        out.push(c);
    }
    out
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The paths named by `#include "path"` lines.
fn includes(clean: &str) -> Vec<String> {
    clean
        .lines()
        .filter_map(|l| {
            let rest = l.trim_start().strip_prefix("#include")?;
            let rest = rest.trim();
            let inner = rest.strip_prefix('"')?;
            Some(inner.split('"').next()?.to_owned())
        })
        .collect()
}

/// `void Name(...)`, `float Name(...)` and the other typed definitions.
fn function_definition(line: &str) -> Option<&str> {
    let t = line.trim_start();
    for kind in ["void", "float", "string", "vector", "entity", "int"] {
        let Some(rest) = t.strip_prefix(kind) else {
            continue;
        };
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let rest = rest.trim_start();
        let end = rest.find(|c: char| !is_word(c)).unwrap_or(rest.len());
        if end > 0 && rest[end..].trim_start().starts_with('(') {
            return Some(&rest[..end]);
        }
    }
    None
}

/// The destinations of the `map("...")` calls on a line.
fn map_calls(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(found) = line[from..].find("map") {
        let at = from + found;
        from = at + 3;
        let before = line[..at].chars().next_back();
        if before.is_some_and(|c| is_word(c) || c == '.' || c == '$') {
            continue;
        }
        let rest = line[at + 3..].trim_start();
        let Some(rest) = rest.strip_prefix('(') else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('"') else {
            continue;
        };
        let dest = rest.split('"').next().unwrap_or_default();
        if !dest.is_empty() && dest.chars().all(|c| is_word(c) || c == '$') {
            out.push(dest.to_owned());
        }
    }
    out
}

/// The verbs a line applies to named entities: `$name.triggerable()`, `$name.nottriggerable()`,
/// `$name.bind(...)` and `trigger("$name")`.
fn verbs(line: &str) -> Vec<(String, char)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(found) = line[from..].find('$') {
        let at = from + found;
        let rest = &line[at + 1..];
        let end = rest.find(|c: char| !is_word(c)).unwrap_or(rest.len());
        from = at + 1 + end;
        if end == 0 {
            continue;
        }
        let name = &rest[..end];
        let after = rest[end..].trim_start();
        let mut verb = None;
        if let Some(method) = after.strip_prefix('.') {
            let method = method.trim_start();
            let m_end = method.find(|c: char| !is_word(c)).unwrap_or(method.len());
            verb = match method[..m_end].to_ascii_lowercase().as_str() {
                "nottriggerable" => Some('D'),
                "triggerable" => Some('E'),
                "bind" => Some('B'),
                _ => None,
            };
        }
        // trigger( "$name" )
        let before = line[..at].trim_end();
        if let Some(before) = before.strip_suffix('"') {
            if let Some(before) = before.trim_end().strip_suffix('(') {
                let before = before.trim_end();
                let word = before
                    .rsplit(|c: char| !is_word(c))
                    .next()
                    .unwrap_or_default();
                if word.eq_ignore_ascii_case("trigger") {
                    verb = Some('F');
                }
            }
        }
        if let Some(v) = verb {
            out.push((name.to_owned(), v));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(text: &str) -> Scan {
        let mut s = Scan::default();
        s.add(&strip_comments(text));
        s
    }

    #[test]
    fn comments_hide_a_dead_level_change() {
        let s = scan(
            "void Live()\n{\n  map( \"next$start\" );\n}\n/*\nvoid Dead()\n{\n  map( \"other$start\" );\n}\n*/\n// map( \"third\" );\n",
        );
        assert_eq!(
            s.calls,
            [MapCall {
                dest: "next$start".into(),
                function: "Live".into()
            }]
        );
        assert!(s.functions.contains("Live") && !s.functions.contains("Dead"));
    }
    #[test]
    fn verbs_are_enable_disable_fire_and_bind() {
        let s = scan(
            "void A()\n{\n$gate.nottriggerable();\n$gate.triggerable();\ntrigger ( \"$door\" );\n$gate.bind( $mover );\n$mover.hide();\nwait( 1 );\n}\n",
        );
        assert_eq!(s.verbs("gate"), "BDE");
        assert_eq!(s.verbs("door"), "F");
        assert_eq!(s.verbs("mover"), "");
        assert_eq!(s.verbs("nobody"), "");
    }
    #[test]
    fn a_map_call_is_a_call_of_the_function_map_only() {
        let s = scan("void F()\n{\n  $x.map( \"a$b\" );\n  goto_map( \"c$d\" );\n  mymap( \"e$f\" );\n  map(\"g\");\n}\n");
        assert_eq!(s.calls.len(), 1);
        assert_eq!(s.calls[0].dest, "g");
    }
    #[test]
    fn typed_definitions_name_the_enclosing_function() {
        assert_eq!(function_definition("void Thing( float x )"), Some("Thing"));
        assert_eq!(function_definition("  float Speed()"), Some("Speed"));
        assert_eq!(function_definition("for( int i = 0; i < 4; i++ )"), None);
        assert_eq!(function_definition("void_x Thing()"), None);
        assert_eq!(function_definition("local float x = 1;"), None);
    }
    #[test]
    fn includes_ignore_commented_lines() {
        let clean = strip_comments("#include \"maps/a.scr\"\n// #include \"missing.scr\"\n");
        assert_eq!(includes(&clean), ["maps/a.scr"]);
    }
}
