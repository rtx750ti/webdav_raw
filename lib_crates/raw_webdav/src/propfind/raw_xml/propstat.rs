use serde::{Deserialize, Serialize};

use crate::propfind::raw_xml::prop::Prop;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct PropStat {
    pub prop: Prop,
    pub status: String,
}
