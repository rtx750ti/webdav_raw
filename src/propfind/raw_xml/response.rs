use serde::{Deserialize, Serialize};

use crate::propfind::raw_xml::propstat::PropStat;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Response {
    pub href: String,
    #[serde(rename = "propstat", default)]
    pub propstat: Vec<PropStat>,
}
