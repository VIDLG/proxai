use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{TranslationError, TranslationResult};

pub(crate) fn from_value<T>(payload: &Value, context: &'static str) -> TranslationResult<T>
where
    T: DeserializeOwned,
{
    match serde_json::from_value(payload.clone()) {
        Ok(value) => Ok(value),
        Err(original) => {
            if let Ok(pretty) = serde_json::to_string_pretty(payload) {
                let mut deserializer = serde_json::Deserializer::from_str(&pretty);
                if let Err(error) = serde_path_to_error::deserialize::<_, T>(&mut deserializer) {
                    let path = error.path().to_string();
                    let error = error.into_inner();
                    return Err(TranslationError::JsonPayload {
                        context,
                        path,
                        message: error.to_string(),
                        line: error.line(),
                        column: error.column(),
                    });
                }
            }

            Err(TranslationError::JsonPayload {
                context,
                path: ".".to_string(),
                message: original.to_string(),
                line: original.line(),
                column: original.column(),
            })
        }
    }
}
