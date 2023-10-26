use base64::display::Base64Display;
use base64::engine::GeneralPurpose;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use rand::{rngs::ThreadRng, Rng};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Span {
    pub trace_id: TraceId,
    #[serde(with = "serde_datetime")]
    pub span_timestamp: DateTime,
}
mod serde_datetime {
    use super::DateTime;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S>(datetime: &DateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(datetime.into_timestamp_nanos())
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<DateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let datetime_i64: i64 = Deserialize::deserialize(deserializer)?;
        Ok(DateTime::from_timestamp_nanos(datetime_i64))
    }
}
#[derive(Clone, Default, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DateTime {
    // Timestamp in nanoseconds.
    pub(crate) timestamp_nanos: i64,
}
impl DateTime {
    /// Create new from UNIX timestamp in nanoseconds.
    pub const fn from_timestamp_nanos(nanoseconds: i64) -> Self {
        Self {
            timestamp_nanos: nanoseconds,
        }
    }

    /// Convert to UNIX timestamp in nanoseconds.
    pub const fn into_timestamp_nanos(self) -> i64 {
        self.timestamp_nanos
    }
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TraceId([u8; 16]);

impl TraceId {
    pub const BASE64_LENGTH: usize = 24;

    pub fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn base64_display(&self) -> Base64Display<'_, '_, GeneralPurpose> {
        Base64Display::new(&self.0, &BASE64_STANDARD)
    }
}

impl Serialize for TraceId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let b64trace_id = BASE64_STANDARD.encode(self.0);
        serializer.serialize_str(&b64trace_id)
    }
}

impl<'de> Deserialize<'de> for TraceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let b64trace_id = String::deserialize(deserializer)?;

        if b64trace_id.len() != TraceId::BASE64_LENGTH {
            let message = format!(
                "base64 trace ID must be {} bytes long, got {}",
                TraceId::BASE64_LENGTH,
                b64trace_id.len()
            );
            return Err(de::Error::custom(message));
        }
        let mut trace_id = [0u8; 16];
        BASE64_STANDARD
            // Using the unchecked version here because otherwise the engine gets the wrong size
            // estimate and fails.
            .decode_slice_unchecked(b64trace_id.as_bytes(), &mut trace_id)
            .map_err(|error| {
                let message = format!("failed to decode base64 trace ID: {:?}", error);
                de::Error::custom(message)
            })?;
        Ok(TraceId(trace_id))
    }
}

impl Span {
    pub fn random(rng: &mut ThreadRng) -> Self {
        Span {
            trace_id: TraceId::random(rng),
            span_timestamp: DateTime::from_timestamp_nanos(rng.gen_range(0..=i64::MAX)),
        }
    }
}

impl TraceId {
    pub fn random(rng: &mut ThreadRng) -> Self {
        let mut id = [0u8; 16];
        rng.fill(&mut id);
        TraceId(id)
    }
}

fn random_spans() -> Vec<Span> {
    let mut rng = rand::thread_rng();
    let length = rng.gen_range(1..=10000);
    (0..length).map(|_| Span::random(&mut rng)).collect()
}
use postcard::{from_bytes, to_allocvec};
fn main() {
    loop {
        let spans = random_spans();

        let output: Vec<u8> = to_allocvec(&spans).unwrap();

        let out: Vec<Span> = from_bytes(&output).unwrap();
        assert_eq!(spans, out);
    }
}
