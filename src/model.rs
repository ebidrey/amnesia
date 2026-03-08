use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project: String,
    pub orchestrator: String,
    pub started_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OpType {
    Decision,
    Bugfix,
    Discovery,
    Pattern,
    Warning,
    Summary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub id: String,
    pub timestamp: String,
    pub agent: String,
    pub op_type: OpType,
    pub title: String,
    pub content: String,
    pub files: Vec<String>,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl std::fmt::Display for OpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            OpType::Decision  => "decision",
            OpType::Bugfix    => "bugfix",
            OpType::Discovery => "discovery",
            OpType::Pattern   => "pattern",
            OpType::Warning   => "warning",
            OpType::Summary   => "summary",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for OpType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "decision"  => Ok(OpType::Decision),
            "bugfix"    => Ok(OpType::Bugfix),
            "discovery" => Ok(OpType::Discovery),
            "pattern"   => Ok(OpType::Pattern),
            "warning"   => Ok(OpType::Warning),
            "summary"   => Ok(OpType::Summary),
            other => Err(format!("unknown type: '{other}'. valid: decision, bugfix, discovery, pattern, warning, summary")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Observation {
        Observation {
            id: "01HX4K2M3N5P6Q7R8S9T0U1V2W".to_string(),
            timestamp: "2026-03-07T14:23:01Z".to_string(),
            agent: "backend-developer".to_string(),
            op_type: OpType::Bugfix,
            title: "Fixed N+1 in product list".to_string(),
            content: "ProductListView was issuing one query per seller.".to_string(),
            files: vec![
                "api/views/products.py".to_string(),
                "api/serializers/product.py".to_string(),
            ],
            tags: vec!["postgresql".to_string(), "django".to_string()],
            session_id: None,
        }
    }

    #[test]
    fn serialize_to_json() {
        let obs = sample();
        let json = serde_json::to_string(&obs).unwrap();

        assert!(json.contains(r#""id":"01HX4K2M3N5P6Q7R8S9T0U1V2W""#));
        assert!(json.contains(r#""op_type":"Bugfix""#));
        assert!(json.contains(r#""agent":"backend-developer""#));
        assert!(json.contains(r#""timestamp":"2026-03-07T14:23:01Z""#));
    }

    #[test]
    fn deserialize_from_json() {
        let json = r#"{"id":"01HX4K2M3N5P6Q7R8S9T0U1V2W","timestamp":"2026-03-07T14:23:01Z","agent":"backend-developer","op_type":"Bugfix","title":"Fixed N+1 in product list","content":"ProductListView was issuing one query per seller.","files":["api/views/products.py","api/serializers/product.py"],"tags":["postgresql","django"]}"#;

        let obs: Observation = serde_json::from_str(json).unwrap();

        assert_eq!(obs.id, "01HX4K2M3N5P6Q7R8S9T0U1V2W");
        assert_eq!(obs.op_type, OpType::Bugfix);
        assert_eq!(obs.agent, "backend-developer");
        assert_eq!(obs.files, vec!["api/views/products.py", "api/serializers/product.py"]);
        assert_eq!(obs.tags, vec!["postgresql", "django"]);
    }

    #[test]
    fn round_trip() {
        let original = sample();
        let json = serde_json::to_string(&original).unwrap();
        let restored: Observation = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn ndjson_multiple_lines() {
        let line1 = r#"{"id":"01A","timestamp":"2026-03-07T10:00:00Z","agent":"api-designer","op_type":"Decision","title":"Use JWT","content":"JWT via cookies","files":[],"tags":[]}"#;
        let line2 = r#"{"id":"01B","timestamp":"2026-03-07T11:00:00Z","agent":"backend-developer","op_type":"Discovery","title":"ORM issue","content":"Lazy loading by default","files":["models.py"],"tags":["orm"]}"#;

        let ndjson = format!("{}\n{}\n", line1, line2);

        let observations: Vec<Observation> = ndjson
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();

        assert_eq!(observations.len(), 2);
        assert_eq!(observations[0].op_type, OpType::Decision);
        assert_eq!(observations[1].op_type, OpType::Discovery);
        assert_eq!(observations[1].files, vec!["models.py"]);
    }

    #[test]
    fn all_op_type_variants_roundtrip() {
        let variants = [
            OpType::Decision,
            OpType::Bugfix,
            OpType::Discovery,
            OpType::Pattern,
            OpType::Warning,
            OpType::Summary,
        ];
        for variant in &variants {
            let json = serde_json::to_string(variant).unwrap();
            let restored: OpType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, &restored);
        }
    }

    #[test]
    fn session_id_none_omitted_from_json() {
        let obs = sample();
        let json = serde_json::to_string(&obs).unwrap();
        assert!(!json.contains("session_id"));
    }

    #[test]
    fn session_id_some_included_in_json() {
        let mut obs = sample();
        obs.session_id = Some("01JNSESSION0000000000000AA".to_string());
        let json = serde_json::to_string(&obs).unwrap();
        assert!(json.contains(r#""session_id":"01JNSESSION0000000000000AA""#));
    }

    #[test]
    fn observation_without_session_id_deserializes_to_none() {
        let json = r#"{"id":"01HX4K2M3N5P6Q7R8S9T0U1V2W","timestamp":"2026-03-07T14:23:01Z","agent":"backend-developer","op_type":"Bugfix","title":"Fixed N+1 in product list","content":"ProductListView was issuing one query per seller.","files":["api/views/products.py","api/serializers/product.py"],"tags":["postgresql","django"]}"#;
        let obs: Observation = serde_json::from_str(json).unwrap();
        assert_eq!(obs.session_id, None);
    }

    #[test]
    fn session_round_trip() {
        let session = Session {
            id: "01JNSESSION0000000000000AA".to_string(),
            project: "amnesia".to_string(),
            orchestrator: "claude".to_string(),
            started_at: "2026-03-08T22:05:00Z".to_string(),
        };
        let json = serde_json::to_string(&session).unwrap();
        let restored: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(session, restored);
    }

    #[test]
    fn session_fields_present_in_json() {
        let session = Session {
            id: "01JNSESSION0000000000000AA".to_string(),
            project: "my-project".to_string(),
            orchestrator: "claude".to_string(),
            started_at: "2026-03-08T22:05:00Z".to_string(),
        };
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains(r#""id":"01JNSESSION0000000000000AA""#));
        assert!(json.contains(r#""project":"my-project""#));
        assert!(json.contains(r#""orchestrator":"claude""#));
        assert!(json.contains(r#""started_at":"2026-03-08T22:05:00Z""#));
    }
}
