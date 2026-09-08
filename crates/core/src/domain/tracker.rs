use serde::de::{Deserializer, Error as DeError, MapAccess, Visitor};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A project type detected on a directory, carrying whatever detail that
/// tracker's detector gathered.
///
/// **Deliberately not an enum.** It was `enum Tracker { Git(GitInfo), … }`,
/// which meant every new detector had to add a variant here — in `domain`,
/// a layer that has no business knowing which detectors exist. Registering a
/// detector now touches nothing outside its own directory but the one line
/// that lists it.
///
/// The payload stays typed where it is produced: a detector builds its own
/// `GitInfo` / `UnrealInfo` struct and hands it to [`Tracker::new`], which
/// serialises it. What is lost is exhaustive matching in `core` — read a field
/// with [`field`](Self::field) and handle its absence, rather than trusting a
/// variant to guarantee it.
///
/// **The wire format is unchanged**, which is why this needed no migration:
/// an externally-tagged enum serialises as `{"Git": {…}}`, and so does this,
/// via the hand-written `Serialize`/`Deserialize` below. `kind` therefore
/// holds the historical *variant* name (`"Git"`, not `"git"`) for the two
/// kinds that predate this change — every stored project already says so.
/// `serializes_as_a_single_key_map` guards it.
#[derive(Debug, Clone, PartialEq)]
pub struct Tracker {
    /// The wire key. For a detector added after this change, pick it once and
    /// never rename it: it is written into every project record.
    pub kind: String,
    pub data: Map<String, Value>,
}

impl Tracker {
    /// Builds a tracker from a detector's own typed info struct.
    ///
    /// Panics only if `info` does not serialise to a JSON object, which for a
    /// plain `#[derive(Serialize)]` struct cannot happen — it is a programming
    /// error in a detector, not a runtime condition.
    pub fn new(kind: impl Into<String>, info: impl Serialize) -> Self {
        let value = serde_json::to_value(info).expect("a tracker payload must serialise");
        let data = match value {
            Value::Object(map) => map,
            other => panic!("a tracker payload must be a JSON object, got {other}"),
        };
        Self {
            kind: kind.into(),
            data,
        }
    }

    /// One field of the payload, if present.
    pub fn field(&self, name: &str) -> Option<&Value> {
        self.data.get(name)
    }

    /// A payload field as a non-empty string, which is what almost every
    /// caller actually wants — `null` and `""` both read as absent.
    pub fn str_field(&self, name: &str) -> Option<&str> {
        self.field(name)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
    }

    /// The payload deserialised back into the detector's own info struct —
    /// the inverse of [`new`](Self::new).
    ///
    /// The typed view survives the container becoming generic: a detector, or
    /// a test, that wants `GitInfo` back asks for it here rather than matching
    /// a variant. `None` if the payload does not fit the requested shape.
    pub fn info<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        serde_json::from_value(Value::Object(self.data.clone())).ok()
    }

    pub fn is(&self, kind: &str) -> bool {
        self.kind.eq_ignore_ascii_case(kind)
    }
}

impl Serialize for Tracker {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.kind, &self.data)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for Tracker {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SingleEntry;

        impl<'de> Visitor<'de> for SingleEntry {
            type Value = Tracker;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a single-key map of tracker kind to its payload")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Tracker, A::Error> {
                let (kind, data): (String, Map<String, Value>) = access
                    .next_entry()?
                    .ok_or_else(|| A::Error::custom("a tracker must carry one kind"))?;
                // A second key would mean two trackers in one slot; refusing is
                // better than silently keeping whichever came first.
                if access.next_entry::<String, Value>()?.is_some() {
                    return Err(A::Error::custom("a tracker must carry exactly one kind"));
                }
                Ok(Tracker { kind, data })
            }
        }

        deserializer.deserialize_map(SingleEntry)
    }
}
