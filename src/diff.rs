use crate::types::Webhook;
use colored::*;

/// Diff result between two webhooks
#[derive(Debug)]
pub struct WebhookDiff {
    pub id1: String,
    pub id2: String,
    pub method_diff: FieldDiff<String>,
    pub path_diff: FieldDiff<String>,
    pub query_diff: FieldDiff<String>,
    pub content_type_diff: FieldDiff<String>,
    pub source_addr_diff: FieldDiff<String>,
    pub timing_diff: Option<String>,
    pub headers_added: Vec<(String, String)>,
    pub headers_removed: Vec<(String, String)>,
    pub headers_changed: Vec<(String, String, String)>,
    pub body_diff: Vec<BodyDiffEntry>,
    pub tags_added: Vec<String>,
    pub tags_removed: Vec<String>,
}

#[derive(Debug)]
pub enum FieldDiff<T: std::fmt::Display> {
    Same(T),
    Different(T, T),
    OnlyInFirst(T),
    OnlyInSecond(T),
}

#[derive(Debug)]
pub struct BodyDiffEntry {
    pub path: String,
    pub left: Option<String>,
    pub right: Option<String>,
    pub status: DiffStatus,
}

#[derive(Debug, PartialEq)]
pub enum DiffStatus {
    Added,
    Removed,
    Changed,
}

/// Compare two webhooks and produce a structured diff
pub fn diff_webhooks(a: &Webhook, b: &Webhook) -> WebhookDiff {
    let method_diff = if a.method == b.method {
        FieldDiff::Same(a.method.clone())
    } else {
        FieldDiff::Different(a.method.clone(), b.method.clone())
    };

    let path_diff = if a.path == b.path {
        FieldDiff::Same(a.path.clone())
    } else {
        FieldDiff::Different(a.path.clone(), b.path.clone())
    };

    let query_diff = if a.query == b.query {
        FieldDiff::Same(a.query.clone())
    } else {
        FieldDiff::Different(a.query.clone(), b.query.clone())
    };

    let content_type_diff = if a.content_type == b.content_type {
        FieldDiff::Same(a.content_type.clone())
    } else {
        FieldDiff::Different(a.content_type.clone(), b.content_type.clone())
    };

    let source_addr_diff = if a.source_addr == b.source_addr {
        FieldDiff::Same(a.source_addr.clone())
    } else {
        FieldDiff::Different(a.source_addr.clone(), b.source_addr.clone())
    };

    let timing_diff = {
        let diff = b.received_at - a.received_at;
        let secs = diff.num_seconds();
        if secs == 0 {
            if diff.num_milliseconds() == 0 {
                None
            } else {
                Some(format!("{} ms apart", diff.num_milliseconds()))
            }
        } else if secs < 60 {
            Some(format!("{} seconds apart", secs))
        } else {
            Some(format!("{}m {}s apart", secs / 60, secs % 60))
        }
    };

    // Header diff
    let mut headers_added = Vec::new();
    let mut headers_removed = Vec::new();
    let mut headers_changed = Vec::new();

    let keys_a: std::collections::BTreeSet<&String> = a.headers.keys().collect();
    let keys_b: std::collections::BTreeSet<&String> = b.headers.keys().collect();

    for key in keys_b.difference(&keys_a) {
        headers_added.push(((*key).clone(), b.headers[*key].clone()));
    }
    for key in keys_a.difference(&keys_b) {
        headers_removed.push(((*key).clone(), a.headers[*key].clone()));
    }
    for key in keys_a.intersection(&keys_b) {
        let va = &a.headers[*key];
        let vb = &b.headers[*key];
        if va != vb {
            headers_changed.push(((*key).clone(), va.clone(), vb.clone()));
        }
    }

    // Tag diff
    let tags_a: std::collections::BTreeSet<&String> = a.tags.iter().collect();
    let tags_b: std::collections::BTreeSet<&String> = b.tags.iter().collect();
    let tags_added: Vec<String> = tags_b.difference(&tags_a).map(|s| (*s).clone()).collect();
    let tags_removed: Vec<String> = tags_a.difference(&tags_b).map(|s| (*s).clone()).collect();

    // Body diff (JSON path-based)
    let body_diff = diff_json_values(&a.body, &b.body);

    WebhookDiff {
        id1: a.id.to_string(),
        id2: b.id.to_string(),
        method_diff,
        path_diff,
        query_diff,
        content_type_diff,
        source_addr_diff,
        timing_diff,
        headers_added,
        headers_removed,
        headers_changed,
        body_diff,
        tags_added,
        tags_removed,
    }
}

/// Recursively diff two JSON values, producing path-based entries
fn diff_json_values(a: &serde_json::Value, b: &serde_json::Value) -> Vec<BodyDiffEntry> {
    let mut entries = Vec::new();
    diff_inner(a, b, "$", &mut entries);
    entries
}

fn diff_inner(
    a: &serde_json::Value,
    b: &serde_json::Value,
    path: &str,
    entries: &mut Vec<BodyDiffEntry>,
) {
    match (a, b) {
        (serde_json::Value::Object(map_a), serde_json::Value::Object(map_b)) => {
            let keys_a: std::collections::BTreeSet<&String> = map_a.keys().collect();
            let keys_b: std::collections::BTreeSet<&String> = map_b.keys().collect();

            for key in keys_b.difference(&keys_a) {
                let sub_path = format!("{}.{}", path, key);
                entries.push(BodyDiffEntry {
                    path: sub_path,
                    left: None,
                    right: Some(format_value(&map_b[*key])),
                    status: DiffStatus::Added,
                });
            }

            for key in keys_a.difference(&keys_b) {
                let sub_path = format!("{}.{}", path, key);
                entries.push(BodyDiffEntry {
                    path: sub_path,
                    left: Some(format_value(&map_a[*key])),
                    right: None,
                    status: DiffStatus::Removed,
                });
            }

            for key in keys_a.intersection(&keys_b) {
                let sub_path = format!("{}.{}", path, key);
                if map_a[*key] != map_b[*key] {
                    diff_inner(&map_a[*key], &map_b[*key], &sub_path, entries);
                }
            }
        }
        (serde_json::Value::Array(arr_a), serde_json::Value::Array(arr_b)) => {
            let max_len = arr_a.len().max(arr_b.len());
            for i in 0..max_len {
                let sub_path = format!("{}[{}]", path, i);
                match (arr_a.get(i), arr_b.get(i)) {
                    (Some(va), Some(vb)) if va == vb => {}
                    (Some(va), Some(vb)) => {
                        diff_inner(va, vb, &sub_path, entries);
                    }
                    (None, Some(vb)) => {
                        entries.push(BodyDiffEntry {
                            path: sub_path,
                            left: None,
                            right: Some(format_value(vb)),
                            status: DiffStatus::Added,
                        });
                    }
                    (Some(va), None) => {
                        entries.push(BodyDiffEntry {
                            path: sub_path,
                            left: Some(format_value(va)),
                            right: None,
                            status: DiffStatus::Removed,
                        });
                    }
                    (None, None) => {}
                }
            }
        }
        _ => {
            if a != b {
                entries.push(BodyDiffEntry {
                    path: path.to_string(),
                    left: Some(format_value(a)),
                    right: Some(format_value(b)),
                    status: DiffStatus::Changed,
                });
            }
        }
    }
}

fn format_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => format!("\"{}\"", s),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Array(arr) => {
            if arr.len() > 5 {
                format!("[{} items]", arr.len())
            } else {
                serde_json::to_string(v).unwrap_or_default()
            }
        }
        serde_json::Value::Object(obj) => {
            format!(
                "{{{}}}",
                obj.keys()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

/// Format and display a webhook diff
pub fn display_diff(diff: &WebhookDiff, format: crate::types::OutputFormat) -> String {
    match format {
        crate::types::OutputFormat::Json => display_diff_json(diff),
        _ => display_diff_text(diff),
    }
}

fn display_diff_text(diff: &WebhookDiff) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "{} Webhook Diff: {} vs {}\n",
        "═══".cyan().bold(),
        diff.id1,
        diff.id2
    ));
    output.push('\n');

    // Basic fields
    output.push_str(&"── Metadata ──\n".cyan().to_string());
    output.push_str(&format_field_diff("Method", &diff.method_diff));
    output.push_str(&format_field_diff("Path", &diff.path_diff));
    output.push_str(&format_field_diff("Query", &diff.query_diff));
    output.push_str(&format_field_diff("Content-Type", &diff.content_type_diff));
    output.push_str(&format_field_diff("Source", &diff.source_addr_diff));
    if let Some(ref timing) = diff.timing_diff {
        output.push_str(&format!("  {}: {}\n", "Timing".green(), timing));
    }
    output.push('\n');

    // Headers
    if !diff.headers_added.is_empty()
        || !diff.headers_removed.is_empty()
        || !diff.headers_changed.is_empty()
    {
        output.push_str(&"── Headers ──\n".cyan().to_string());
        for (key, value) in &diff.headers_added {
            output.push_str(&format!("  {} {}: {}\n", "+".green().bold(), key, value));
        }
        for (key, value) in &diff.headers_removed {
            output.push_str(&format!("  {} {}: {}\n", "-".red().bold(), key, value));
        }
        for (key, old_val, new_val) in &diff.headers_changed {
            output.push_str(&format!(
                "  {} {}: {} → {}\n",
                "~".yellow().bold(),
                key,
                old_val.red(),
                new_val.green()
            ));
        }
        output.push('\n');
    }

    // Tags
    if !diff.tags_added.is_empty() || !diff.tags_removed.is_empty() {
        output.push_str(&"── Tags ──\n".cyan().to_string());
        for tag in &diff.tags_added {
            output.push_str(&format!("  {} {}\n", "+".green().bold(), tag));
        }
        for tag in &diff.tags_removed {
            output.push_str(&format!("  {} {}\n", "-".red().bold(), tag));
        }
        output.push('\n');
    }

    // Body
    if !diff.body_diff.is_empty() {
        output.push_str(&"── Body ──\n".cyan().to_string());
        let max_path_len = diff
            .body_diff
            .iter()
            .map(|e| e.path.len())
            .max()
            .unwrap_or(20)
            .min(60);

        for entry in &diff.body_diff {
            let status_marker = match entry.status {
                DiffStatus::Added => "+".green().bold(),
                DiffStatus::Removed => "-".red().bold(),
                DiffStatus::Changed => "~".yellow().bold(),
            };

            match &entry.left {
                Some(left) if entry.status == DiffStatus::Changed => {
                    let right = entry.right.as_deref().unwrap_or("");
                    output.push_str(&format!(
                        "  {} {:width$}  {}  →  {}\n",
                        status_marker,
                        entry.path,
                        left,
                        right,
                        width = max_path_len
                    ));
                }
                Some(left) => {
                    output.push_str(&format!(
                        "  {} {:width$}  {}\n",
                        status_marker,
                        entry.path,
                        left,
                        width = max_path_len
                    ));
                }
                None => {
                    let right = entry.right.as_deref().unwrap_or("");
                    output.push_str(&format!(
                        "  {} {:width$}  {}\n",
                        status_marker,
                        entry.path,
                        right,
                        width = max_path_len
                    ));
                }
            }
        }
    } else {
        output.push_str(&"Body: identical\n".green().to_string());
    }

    output
}

fn format_field_diff<T: std::fmt::Display>(label: &str, diff: &FieldDiff<T>) -> String {
    match diff {
        FieldDiff::Same(val) => format!("  {}: {}\n", label, val),
        FieldDiff::Different(a, b) => {
            format!(
                "  {}: {} → {}\n",
                label,
                a.to_string().red(),
                b.to_string().green()
            )
        }
        FieldDiff::OnlyInFirst(val) => {
            format!("  {}: {} {}\n", label, "🗑️", val.to_string().red())
        }
        FieldDiff::OnlyInSecond(val) => {
            format!("  {}: {} {}\n", label, "✨", val.to_string().green())
        }
    }
}

fn display_diff_json(diff: &WebhookDiff) -> String {
    let mut map = serde_json::Map::new();

    map.insert(
        "id1".to_string(),
        serde_json::Value::String(diff.id1.clone()),
    );
    map.insert(
        "id2".to_string(),
        serde_json::Value::String(diff.id2.clone()),
    );

    map.insert("method".to_string(), field_diff_to_json(&diff.method_diff));
    map.insert("path".to_string(), field_diff_to_json(&diff.path_diff));
    map.insert("query".to_string(), field_diff_to_json(&diff.query_diff));
    map.insert(
        "content_type".to_string(),
        field_diff_to_json(&diff.content_type_diff),
    );
    map.insert(
        "source_addr".to_string(),
        field_diff_to_json(&diff.source_addr_diff),
    );

    if let Some(ref timing) = diff.timing_diff {
        map.insert(
            "timing".to_string(),
            serde_json::Value::String(timing.clone()),
        );
    }

    // Headers
    let headers = serde_json::json!({
        "added": diff.headers_added.iter().map(|(k, v)| serde_json::json!({k: v})).collect::<Vec<_>>(),
        "removed": diff.headers_removed.iter().map(|(k, v)| serde_json::json!({k: v})).collect::<Vec<_>>(),
        "changed": diff.headers_changed.iter().map(|(k, o, n)| serde_json::json!({"key": k, "old": o, "new": n})).collect::<Vec<_>>(),
    });
    map.insert("headers".to_string(), headers);

    // Tags
    map.insert(
        "tags".to_string(),
        serde_json::json!({
            "added": diff.tags_added,
            "removed": diff.tags_removed,
        }),
    );

    // Body
    let body_diffs: Vec<serde_json::Value> = diff
        .body_diff
        .iter()
        .map(|e| {
            serde_json::json!({
                "path": e.path,
                "left": e.left,
                "right": e.right,
                "status": match e.status {
                    DiffStatus::Added => "added",
                    DiffStatus::Removed => "removed",
                    DiffStatus::Changed => "changed",
                }
            })
        })
        .collect();
    map.insert(
        "body_diff".to_string(),
        serde_json::Value::Array(body_diffs),
    );

    serde_json::to_string_pretty(&serde_json::Value::Object(map)).unwrap_or_default()
}

fn field_diff_to_json<T: std::fmt::Display>(diff: &FieldDiff<T>) -> serde_json::Value {
    match diff {
        FieldDiff::Same(val) => serde_json::json!({"status": "same", "value": val.to_string()}),
        FieldDiff::Different(a, b) => {
            serde_json::json!({"status": "changed", "old": a.to_string(), "new": b.to_string()})
        }
        FieldDiff::OnlyInFirst(val) => {
            serde_json::json!({"status": "removed", "value": val.to_string()})
        }
        FieldDiff::OnlyInSecond(val) => {
            serde_json::json!({"status": "added", "value": val.to_string()})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Webhook;
    use serde_json::json;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn make_webhook(
        method: &str,
        path: &str,
        body: serde_json::Value,
        headers: HashMap<String, String>,
        tags: Vec<String>,
    ) -> Webhook {
        Webhook {
            id: Uuid::new_v4(),
            method: method.to_string(),
            path: path.to_string(),
            query: String::new(),
            headers,
            body,
            raw_body: None,
            content_type: "application/json".to_string(),
            source_addr: "127.0.0.1".to_string(),
            received_at: chrono::Utc::now(),
            tags,
        }
    }

    #[test]
    fn test_same_webhooks_no_diff() {
        let headers = HashMap::from([("content-type".into(), "application/json".into())]);
        let body = json!({"event": "push", "ref": "main"});
        let a = make_webhook("POST", "/hook", body.clone(), headers.clone(), vec![]);
        let b = Webhook {
            id: Uuid::new_v4(),
            received_at: chrono::Utc::now(),
            ..a.clone()
        };
        let diff = diff_webhooks(&a, &b);
        assert!(matches!(diff.method_diff, FieldDiff::Same(_)));
        assert!(matches!(diff.path_diff, FieldDiff::Same(_)));
        assert!(diff.body_diff.is_empty());
        assert!(diff.headers_added.is_empty());
        assert!(diff.headers_changed.is_empty());
    }

    #[test]
    fn test_different_method() {
        let a = make_webhook("POST", "/hook", json!({}), HashMap::new(), vec![]);
        let b = make_webhook("GET", "/hook", json!({}), HashMap::new(), vec![]);
        let diff = diff_webhooks(&a, &b);
        assert!(matches!(diff.method_diff, FieldDiff::Different(_, _)));
        if let FieldDiff::Different(old, new) = &diff.method_diff {
            assert_eq!(old, "POST");
            assert_eq!(new, "GET");
        }
    }

    #[test]
    fn test_body_added_field() {
        let a = make_webhook(
            "POST",
            "/hook",
            json!({"name": "test"}),
            HashMap::new(),
            vec![],
        );
        let b = make_webhook(
            "POST",
            "/hook",
            json!({"name": "test", "age": 30}),
            HashMap::new(),
            vec![],
        );
        let diff = diff_webhooks(&a, &b);
        let added: Vec<_> = diff
            .body_diff
            .iter()
            .filter(|e| e.status == DiffStatus::Added)
            .collect();
        assert_eq!(added.len(), 1);
        assert!(added[0].path.contains("age"));
    }

    #[test]
    fn test_body_removed_field() {
        let a = make_webhook(
            "POST",
            "/hook",
            json!({"name": "test", "age": 30}),
            HashMap::new(),
            vec![],
        );
        let b = make_webhook(
            "POST",
            "/hook",
            json!({"name": "test"}),
            HashMap::new(),
            vec![],
        );
        let diff = diff_webhooks(&a, &b);
        let removed: Vec<_> = diff
            .body_diff
            .iter()
            .filter(|e| e.status == DiffStatus::Removed)
            .collect();
        assert_eq!(removed.len(), 1);
        assert!(removed[0].path.contains("age"));
    }

    #[test]
    fn test_body_changed_field() {
        let a = make_webhook(
            "POST",
            "/hook",
            json!({"name": "test"}),
            HashMap::new(),
            vec![],
        );
        let b = make_webhook(
            "POST",
            "/hook",
            json!({"name": "updated"}),
            HashMap::new(),
            vec![],
        );
        let diff = diff_webhooks(&a, &b);
        let changed: Vec<_> = diff
            .body_diff
            .iter()
            .filter(|e| e.status == DiffStatus::Changed)
            .collect();
        assert_eq!(changed.len(), 1);
        assert!(changed[0].path.contains("name"));
    }

    #[test]
    fn test_header_diffs() {
        let headers_a = HashMap::from([
            ("x-request-id".into(), "abc".into()),
            ("user-agent".into(), "curl".into()),
        ]);
        let headers_b = HashMap::from([
            ("x-request-id".into(), "def".into()),
            ("authorization".into(), "Bearer token".into()),
        ]);
        let a = make_webhook("POST", "/hook", json!({}), headers_a, vec![]);
        let b = make_webhook("POST", "/hook", json!({}), headers_b, vec![]);
        let diff = diff_webhooks(&a, &b);
        assert_eq!(diff.headers_removed.len(), 1);
        assert!(diff.headers_removed.iter().any(|(k, _)| k == "user-agent"));
        assert_eq!(diff.headers_added.len(), 1);
        assert!(diff.headers_added.iter().any(|(k, _)| k == "authorization"));
        assert_eq!(diff.headers_changed.len(), 1);
        assert!(
            diff.headers_changed
                .iter()
                .any(|(k, _, _)| k == "x-request-id")
        );
    }

    #[test]
    fn test_tag_diffs() {
        let a = make_webhook(
            "POST",
            "/hook",
            json!({}),
            HashMap::new(),
            vec!["tag1".into(), "tag2".into()],
        );
        let b = make_webhook(
            "POST",
            "/hook",
            json!({}),
            HashMap::new(),
            vec!["tag1".into(), "tag3".into()],
        );
        let diff = diff_webhooks(&a, &b);
        assert_eq!(diff.tags_added, vec!["tag3"]);
        assert_eq!(diff.tags_removed, vec!["tag2"]);
    }

    #[test]
    fn test_nested_body_diff() {
        let a = make_webhook(
            "POST",
            "/hook",
            json!({"user": {"name": "Alice", "role": "admin"}}),
            HashMap::new(),
            vec![],
        );
        let b = make_webhook(
            "POST",
            "/hook",
            json!({"user": {"name": "Bob", "role": "admin", "email": "b@x.com"}}),
            HashMap::new(),
            vec![],
        );
        let diff = diff_webhooks(&a, &b);
        let changed: Vec<_> = diff
            .body_diff
            .iter()
            .filter(|e| e.status == DiffStatus::Changed)
            .collect();
        let added: Vec<_> = diff
            .body_diff
            .iter()
            .filter(|e| e.status == DiffStatus::Added)
            .collect();
        assert!(changed.iter().any(|e| e.path.contains("name")));
        assert!(added.iter().any(|e| e.path.contains("email")));
    }

    #[test]
    fn test_json_output() {
        let a = make_webhook("POST", "/a", json!({"x": 1}), HashMap::new(), vec![]);
        let b = make_webhook("GET", "/b", json!({"x": 2}), HashMap::new(), vec![]);
        let diff = diff_webhooks(&a, &b);
        let json = display_diff_json(&diff);
        assert!(json.contains("\"id1\""));
        assert!(json.contains("\"id2\""));
        assert!(json.contains("\"method\""));
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn test_identical_bodies() {
        let a = make_webhook(
            "POST",
            "/hook",
            json!({"a": 1, "b": 2}),
            HashMap::new(),
            vec![],
        );
        let b = Webhook {
            id: Uuid::new_v4(),
            received_at: chrono::Utc::now(),
            ..a.clone()
        };
        let diff = diff_webhooks(&a, &b);
        assert!(diff.body_diff.is_empty());
    }

    #[test]
    fn test_path_diff() {
        let a = make_webhook("POST", "/v1/hook", json!({}), HashMap::new(), vec![]);
        let b = make_webhook("POST", "/v2/hook", json!({}), HashMap::new(), vec![]);
        let diff = diff_webhooks(&a, &b);
        assert!(matches!(diff.path_diff, FieldDiff::Different(_, _)));
    }

    #[test]
    fn test_array_diff() {
        let a = make_webhook(
            "POST",
            "/hook",
            json!({"items": [1, 2, 3]}),
            HashMap::new(),
            vec![],
        );
        let b = make_webhook(
            "POST",
            "/hook",
            json!({"items": [1, 4, 3, 5]}),
            HashMap::new(),
            vec![],
        );
        let diff = diff_webhooks(&a, &b);
        let changed: Vec<_> = diff
            .body_diff
            .iter()
            .filter(|e| e.status == DiffStatus::Changed)
            .collect();
        let added: Vec<_> = diff
            .body_diff
            .iter()
            .filter(|e| e.status == DiffStatus::Added)
            .collect();
        assert!(changed.iter().any(|e| e.path.contains("[1]")));
        assert!(added.iter().any(|e| e.path.contains("[3]")));
    }
}
