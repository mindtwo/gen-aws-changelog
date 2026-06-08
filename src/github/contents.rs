use crate::error::Result;
use crate::github::GithubClient;
use base64_compat::decode as b64_decode;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ContentsResponse {
    content: String,
    encoding: String,
}

/// Fetches a file from the repo's default branch and returns its bytes.
pub async fn fetch_file(client: &GithubClient, path: &str) -> Result<Vec<u8>> {
    let url = client.url(&format!("contents/{path}"));
    let resp = client.http().get(&url).send().await?;
    let status = resp.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("{path} not found in {}", client.repo());
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("GitHub contents failed ({status}): {body}");
    }
    let parsed: ContentsResponse = resp.json().await?;
    if parsed.encoding != "base64" {
        anyhow::bail!("unexpected encoding from GitHub: {}", parsed.encoding);
    }
    // GitHub wraps base64 in newlines; strip them before decoding.
    let cleaned: String = parsed.content.chars().filter(|c| !c.is_whitespace()).collect();
    b64_decode(&cleaned).map_err(|e| anyhow::anyhow!("base64 decode failed: {e}"))
}

/// Tiny in-tree base64 decoder so we don't pull a separate crate just for this.
mod base64_compat {
    pub fn decode(s: &str) -> Result<Vec<u8>, &'static str> {
        // Standard base64 alphabet
        const T: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut lut = [255u8; 256];
        for (i, c) in T.iter().enumerate() {
            lut[*c as usize] = i as u8;
        }
        let bytes: Vec<u8> = s.bytes().filter(|b| *b != b'=').collect();
        if bytes.iter().any(|b| lut[*b as usize] == 255) {
            return Err("invalid base64 character");
        }
        let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
        for chunk in bytes.chunks(4) {
            let n0 = lut[chunk[0] as usize] as u32;
            let n1 = if chunk.len() > 1 {
                lut[chunk[1] as usize] as u32
            } else {
                0
            };
            let n2 = if chunk.len() > 2 {
                lut[chunk[2] as usize] as u32
            } else {
                0
            };
            let n3 = if chunk.len() > 3 {
                lut[chunk[3] as usize] as u32
            } else {
                0
            };
            let triple = (n0 << 18) | (n1 << 12) | (n2 << 6) | n3;
            out.push(((triple >> 16) & 0xFF) as u8);
            if chunk.len() > 2 {
                out.push(((triple >> 8) & 0xFF) as u8);
            }
            if chunk.len() > 3 {
                out.push((triple & 0xFF) as u8);
            }
        }
        Ok(out)
    }

    #[cfg(test)]
    mod tests {
        use super::decode;

        #[test]
        fn roundtrip_simple() {
            assert_eq!(decode("aGVsbG8=").unwrap(), b"hello");
            assert_eq!(decode("Zm9vYmFy").unwrap(), b"foobar");
        }
    }
}
