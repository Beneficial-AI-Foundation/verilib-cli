use anyhow::Result;
use reqwest::Response;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct ApiErrorResponse {
    error: bool,
    data: ApiErrorData,
}

#[derive(Deserialize, Debug)]
struct ApiErrorData {
    code: u16,
    message: String,
}

pub async fn handle_api_error(response: Response) -> Result<String> {
    let status = response.status();
    
    let response_text = match response.text().await {
        Ok(text) => text,
        Err(_) => return Ok(format!("API request failed with status: {}", status)),
    };
    
    if let Ok(error_response) = serde_json::from_str::<ApiErrorResponse>(&response_text) {
        if error_response.error {
            return Ok(format!(
                "API error ({}): {}",
                error_response.data.code,
                error_response.data.message
            ));
        }
    }
    
    if !response_text.is_empty() {
        Ok(format!("API request failed with status: {} - {}", status, response_text))
    } else {
        Ok(format!("API request failed with status: {} - Unable to read error response", status))
    }
}
