use super::candidate::{AdapterResult, DiagnosticIssue, FieldCandidate};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::{fs, path::Path};

#[derive(Clone, Debug, Default)]
pub struct AuthCandidates {
    pub username: Option<FieldCandidate<String>>,
    pub subscription: Option<FieldCandidate<String>>,
}

pub fn read_auth_candidates(root: &Path) -> AdapterResult<AuthCandidates> {
    let mut result = AdapterResult::default();
    let path = root.join("auth.json");
    let observed_at = fs::metadata(&path)
        .and_then(|metadata| metadata.modified())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now());
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => {
            result
                .issues
                .push(DiagnosticIssue::new("auth-file-missing", "auth.json"));
            return result;
        }
    };
    let auth: Value = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(_) => {
            result
                .issues
                .push(DiagnosticIssue::new("auth-json-invalid", "auth.json"));
            return result;
        }
    };
    let Some(token) = auth.pointer("/tokens/id_token").and_then(Value::as_str) else {
        result
            .issues
            .push(DiagnosticIssue::new("auth-token-missing", "auth.json"));
        return result;
    };
    let Some(payload) = token.split('.').nth(1) else {
        result
            .issues
            .push(DiagnosticIssue::new("auth-token-invalid", "auth.json"));
        return result;
    };
    let claims: Value = match URL_SAFE_NO_PAD
        .decode(payload)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    {
        Some(claims) => claims,
        None => {
            result
                .issues
                .push(DiagnosticIssue::new("auth-token-invalid", "auth.json"));
            return result;
        }
    };

    let username = claims
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| claims.get("email").and_then(Value::as_str));
    if let Some(value) = username {
        result.value.username = Some(FieldCandidate::new(
            value.to_owned(),
            "auth",
            observed_at,
            0.90,
        ));
    }
    if let Some(value) = claims
        .get("https://api.openai.com/auth/chatgpt_plan_type")
        .and_then(Value::as_str)
    {
        result.value.subscription = Some(FieldCandidate::new(
            value.to_owned(),
            "auth",
            observed_at,
            0.80,
        ));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::read_auth_candidates;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use serde_json::json;
    use std::fs;

    fn write_auth(root: &std::path::Path, token: &str) {
        fs::write(
            root.join("auth.json"),
            json!({ "tokens": { "id_token": token } }).to_string(),
        )
        .unwrap();
    }

    #[test]
    fn reads_username_and_subscription_from_valid_claims() {
        let temp = tempfile::tempdir().unwrap();
        let payload = URL_SAFE_NO_PAD.encode(
            json!({
                "name": "Test User",
                "https://api.openai.com/auth/chatgpt_plan_type": "plus"
            })
            .to_string(),
        );
        write_auth(temp.path(), &format!("header.{payload}.signature"));

        let result = read_auth_candidates(temp.path());

        assert_eq!(result.value.username.unwrap().value, "Test User");
        assert_eq!(result.value.subscription.unwrap().value, "plus");
    }

    #[test]
    fn token_never_enters_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        write_auth(temp.path(), "header.secret_payload.signature");

        let result = read_auth_candidates(temp.path());
        let serialized = serde_json::to_string(&result.issues).unwrap();

        assert!(!serialized.contains("secret_payload"));
        assert!(result
            .issues
            .iter()
            .any(|issue| issue.code == "auth-token-invalid"));
    }
}
