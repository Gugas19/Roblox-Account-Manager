//! Dates the way Newtonsoft.Json writes .NET DateTime values: "2026-10-11T13:23:46.7668383+01:00" or "0001-01-01T00:00:00"

use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, TimeZone};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NetDate(pub Option<DateTime<FixedOffset>>);

impl NetDate {
    pub fn now() -> NetDate {
        NetDate(Some(Local::now().fixed_offset()))
    }

    pub fn min() -> NetDate {
        NetDate(None)
    }

    /// Days since this date (0 for DateTime.MinValue / unknown dates, like a never used account would be "very old" otherwise)
    pub fn days_ago(&self) -> f64 {
        match self.0 {
            Some(d) => (Local::now().fixed_offset() - d).num_seconds() as f64 / 86400.0,
            None => f64::INFINITY,
        }
    }

    pub fn parse(text: &str) -> NetDate {
        let text = text.trim();

        if let Ok(d) = DateTime::parse_from_rfc3339(text) {
            return NetDate(Some(d));
        }

        // no offset: .NET "Unspecified" kind, treat as local time; the minimum value means "never"
        for format in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S"] {
            if let Ok(naive) = NaiveDateTime::parse_from_str(text, format) {
                if naive.and_utc().timestamp() <= -62135596800 {
                    return NetDate(None);
                }
                if let Some(local) = Local.from_local_datetime(&naive).single() {
                    return NetDate(Some(local.fixed_offset()));
                }
            }
        }

        NetDate(None)
    }

    pub fn to_net_string(&self) -> String {
        match self.0 {
            // .NET writes 7 digits of fractions (100 ns ticks), chrono has no %.7f
            Some(d) => format!("{}.{:07}{}", d.format("%Y-%m-%dT%H:%M:%S"), d.timestamp_subsec_nanos() / 100, d.format("%:z")),
            None => "0001-01-01T00:00:00".to_string(),
        }
    }

    /// Seconds since 1970 with milliseconds, like the C# ToRobloxTick (web API "LastUsed")
    pub fn roblox_tick(&self) -> f64 {
        self.0.map(|d| d.timestamp_millis() as f64 / 1000.0).unwrap_or(-62135596800.0)
    }
}

impl Serialize for NetDate {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_net_string())
    }
}

impl<'de> Deserialize<'de> for NetDate {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = Option::<String>::deserialize(d)?;
        Ok(value.map(|v| NetDate::parse(&v)).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_dotnet_dates() {
        let d = NetDate::parse("2026-10-11T13:23:46.7668383+01:00");
        assert_eq!(d.to_net_string(), "2026-10-11T13:23:46.7668383+01:00");
        assert_eq!(NetDate::parse("0001-01-01T00:00:00").to_net_string(), "0001-01-01T00:00:00");
        assert!(NetDate::parse("0001-01-01T00:00:00").days_ago().is_infinite());
        assert!(NetDate::now().days_ago() < 0.01);
        assert!(NetDate::parse("2020-01-01T00:00:00").0.is_some());
    }
}
