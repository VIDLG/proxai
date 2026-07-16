use serde_json::json;

use super::VoiceIdsOrCustomVoice;

#[test]
fn voice_accepts_openai_builtin_and_custom_voice_references() {
    assert_eq!(
        serde_json::from_value::<VoiceIdsOrCustomVoice>(json!("cedar")).unwrap(),
        VoiceIdsOrCustomVoice::BuiltIn("cedar".to_string())
    );
    assert_eq!(
        serde_json::from_value::<VoiceIdsOrCustomVoice>(json!({ "id": "voice_123" })).unwrap(),
        VoiceIdsOrCustomVoice::Custom {
            id: "voice_123".to_string(),
        }
    );
    assert!(serde_json::from_value::<VoiceIdsOrCustomVoice>(json!(null)).is_err());
}
