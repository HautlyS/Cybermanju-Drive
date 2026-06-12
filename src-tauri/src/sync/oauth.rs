// Cybermanju Drive — OAuth2 Token Management
// Handles token refresh for Google Drive, Google Photos, GitHub, and GitLab

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

/// OAuth2 token response from providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

/// Stored OAuth2 credentials with refresh capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>, // Unix timestamp
    pub client_id: String,
    pub client_secret: Option<String>,
}

impl OAuthCredentials {
    /// Check if the token is expired or will expire within `buffer_seconds`
    pub fn is_expired(&self, buffer_seconds: u64) -> bool {
        match self.expires_at {
            Some(expires) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                now + buffer_seconds >= expires
            }
            None => false, // No expiry info, assume valid
        }
    }

    /// Refresh the access token using the refresh_token
    pub fn refresh(&mut self, token_url: &str) -> Result<(), String> {
        let refresh_token = self
            .refresh_token
            .as_ref()
            .ok_or("No refresh token available")?;

        let client = Client::builder()
            .user_agent("CybermanjuDrive/0.1")
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let mut params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
        ];

        if let Some(ref client_id) = self.client_id.as_str() {
            params.push(("client_id", client_id));
        }

        if let Some(ref client_secret) = self.client_secret {
            params.push(("client_secret", client_secret.as_str()));
        }

        let resp = client
            .post(token_url)
            .form(&params)
            .send()
            .map_err(|e| format!("Token refresh request failed: {}", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| format!("Failed to read token refresh response: {}", e))?;

        if status < 200 || status >= 300 {
            return Err(format!("Token refresh failed ({}): {}", status, body));
        }

        let token_resp: TokenResponse = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse token refresh response: {}", e))?;

        self.access_token = token_resp.access_token;

        if let Some(refresh) = token_resp.refresh_token {
            self.refresh_token = Some(refresh);
        }

        if let Some(expires_in) = token_resp.expires_in {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            self.expires_at = Some(now + expires_in);
        }

        Ok(())
    }
}

/// Google OAuth2 token refresh
pub fn refresh_google_token(credentials: &mut OAuthCredentials) -> Result<(), String> {
    credentials.refresh("https://oauth2.googleapis.com/token")
}

/// GitHub OAuth2 token refresh (GitHub doesn't actually support refresh tokens for PATs,
/// but this works for OAuth apps)
pub fn refresh_github_token(credentials: &mut OAuthCredentials) -> Result<(), String> {
    credentials.refresh("https://github.com/login/oauth/access_token")
}

/// GitLab OAuth2 token refresh
pub fn refresh_gitlab_token(
    credentials: &mut OAuthCredentials,
    instance_url: Option<&str>,
) -> Result<(), String> {
    let base = instance_url.unwrap_or("https://gitlab.com");
    let token_url = format!("{}/oauth/token", base.trim_end_matches('/'));
    credentials.refresh(&token_url)
}

/// Get a valid access token, refreshing if necessary
pub fn get_valid_token(
    credentials: &mut OAuthCredentials,
    token_url: &str,
    buffer_seconds: u64,
) -> Result<String, String> {
    if credentials.is_expired(buffer_seconds) {
        credentials.refresh(token_url)?;
    }
    Ok(credentials.access_token.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_expired() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut creds = OAuthCredentials {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: Some(now + 3600), // Expires in 1 hour
            client_id: "test".to_string(),
            client_secret: None,
        };

        // Not expired with 5 minute buffer
        assert!(!creds.is_expired(300));

        // Expired with 2 hour buffer
        assert!(creds.is_expired(7200));

        // No expiry = not expired
        creds.expires_at = None;
        assert!(!creds.is_expired(300));
    }
}
