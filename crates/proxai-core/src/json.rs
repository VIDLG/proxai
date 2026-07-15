use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::JsonPayloadError;

pub(crate) fn deserialize_value<T>(
    payload: &Value,
    context: &'static str,
) -> Result<T, JsonPayloadError>
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
                    return Err(JsonPayloadError::new(context, path, error.into_inner()));
                }
            }

            Err(JsonPayloadError::new(context, ".", original))
        }
    }
}
