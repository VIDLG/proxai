use serde::Serialize;

#[derive(Serialize)]
struct CitationCharLocation {
    cited_text: String,
    document_index: u32,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum TextCitation {
    #[serde(rename = "char_location")]
    CharLocation(CitationCharLocation),
}

fn main() {
    let c = CitationCharLocation {
        cited_text: "hello".into(),
        document_index: 0,
    };

    let wrapped = TextCitation::CharLocation(c);
    println!("通过 enum:  {}", serde_json::to_string(&wrapped).unwrap());

    let alone = CitationCharLocation {
        cited_text: "hello".into(),
        document_index: 0,
    };
    println!("单独 struct: {}", serde_json::to_string(&alone).unwrap());
}
