use serde::{Deserialize, Serialize};
use std::str::FromStr;

use derive_is_enum_variant::is_enum_variant;
#[derive(Clone, Debug, Deserialize, Serialize, Hash, PartialEq, Eq, is_enum_variant)]
/// Target Platform based
pub enum PBXTargetPlatform {
    /// iOS Platform
    #[serde(rename = "iOS")]
    IOS,
    /// watchOS Platform
    #[serde(rename = "watchOS")]
    WatchOS,
    /// tvOs Platform
    #[serde(rename = "tvOS")]
    TvOS,
    /// macOS Platform
    #[serde(rename = "macOS")]
    MacOS,
    /// visionOS Platform
    #[serde(rename = "xrOS")]
    XrOS,
    /// Unknown or not support platform
    Unknown,
}

impl Default for PBXTargetPlatform {
    fn default() -> Self {
        Self::Unknown
    }
}

impl PBXTargetPlatform {
    /// Get Target Platfrom from sdkroot
    pub fn from_sdk_root(sdk_root: &str) -> Self {
        match sdk_root {
            "iphoneos" => Self::IOS,
            "macosx" => Self::MacOS,
            "appletvos" => Self::TvOS,
            "watchos" => Self::WatchOS,
            "xros" => Self::XrOS,
            _ => Self::Unknown,
        }
    }
    /// Get PBXTargetPlatform from simulator identifer
    pub fn from_identifer(identifer: &str) -> Self {
        let name = identifer.replace("com.apple.CoreSimulator.SimRuntime.", "");
        let platform_str = name.split("-").next().unwrap().to_string();
        match Self::from_str(&platform_str) {
            Ok(res) => res,
            Err(e) => {
                tracing::error!("Platfrom from str: {e}");
                Self::Unknown
            }
        }
    }
}

impl FromStr for PBXTargetPlatform {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, String> {
        match s {
            "iOS" => Ok(Self::IOS),
            "watchOS" => Ok(Self::WatchOS),
            "tvOS" => Ok(Self::TvOS),
            "macOS" => Ok(Self::MacOS),
            "xrOS" => Ok(Self::XrOS),
            _ => Ok(Self::Unknown),
        }
    }
}

impl ToString for PBXTargetPlatform {
    fn to_string(&self) -> String {
        match self {
            Self::IOS => "iOS",
            Self::WatchOS => "watchOS",
            Self::TvOS => "tvOS",
            Self::MacOS => "macOS",
            Self::XrOS => "xrOS",
            _ => "",
        }
        .into()
    }
}
