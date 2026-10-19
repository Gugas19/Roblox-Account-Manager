//! Everything the interface can ask the app to do, one file per part of the interface.

pub mod antiafk;
pub mod startup;
pub mod protection;
pub mod accounts;
pub mod joining;
pub mod games;
pub mod servers;
pub mod avatar;
pub mod account_utils;
pub mod browsing;
pub mod settings;
pub mod updates;
pub mod backup;
pub mod history;


use crate::app::{app, AccountView, App};
use crate::core::accounts::{Account, LoadState};
use crate::core::lang;
use crate::platform::win as winutil;
use crate::services::browser::{self, Browser, LaunchOptions};
use crate::services::antiafk::cpu;
use crate::services::launcher::{self, JoinOptions, JoinResult};
use crate::services::webserver;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

pub(super) type R<T> = Result<T, String>;

pub(super) fn e<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}

pub(super) fn window_rect(account: &Account) -> Option<(i32, i32, i32, i32)> {
    Some((account.field("Window_Position_X").parse().ok()?, account.field("Window_Position_Y").parse().ok()?, account.field("Window_Width").parse().ok()?, account.field("Window_Height").parse().ok()?))
}

#[allow(unused_imports)]
use startup::*;
#[allow(unused_imports)]
use protection::*;
#[allow(unused_imports)]
use accounts::*;
#[allow(unused_imports)]
use joining::*;
#[allow(unused_imports)]
use games::*;
#[allow(unused_imports)]
use servers::*;
#[allow(unused_imports)]
use avatar::*;
#[allow(unused_imports)]
use account_utils::*;
#[allow(unused_imports)]
use browsing::*;
#[allow(unused_imports)]
use settings::*;
