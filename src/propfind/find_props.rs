use std::collections::BTreeSet;

use quick_xml::{
    Writer,
    events::{BytesEnd, BytesStart, Event},
};

/// 起名跟协议里的函数名一致
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FindProp {
    Resourcetype,
    Creationdate,
    Getetag,
    Getlastmodified,
    Getcontenttype,
}

impl FindProp {
    fn tag_name(self) -> &'static str {
        match self {
            Self::Resourcetype => "resourcetype",
            Self::Creationdate => "creationdate",
            Self::Getetag => "getetag",
            Self::Getlastmodified => "getlastmodified",
            Self::Getcontenttype => "getcontenttype",
        }
    }

    /// 传入Writer转成标准xml结构
    fn write_xml<W: std::io::Write>(self, writer: &mut Writer<W>) -> Result<(), quick_xml::Error> {
        let tag_name = format!("D:{}", self.tag_name());

        writer.write_event(Event::Empty(BytesStart::new(tag_name.as_str())))?;

        Ok(())
    }
}

pub enum PropFindSelector {
    AllProp,
    PropName,
    Props(BTreeSet<FindProp>), // 保持稳定顺序自动去重
}

impl PropFindSelector {
    fn write_xml<W: std::io::Write>(&self, writer: &mut Writer<W>) -> Result<(), quick_xml::Error> {
        match self {
            PropFindSelector::AllProp => {
                writer.write_event(Event::Empty(BytesStart::new("D:allprop")))?;
            }
            PropFindSelector::PropName => {
                writer.write_event(Event::Empty(BytesStart::new("D:propname")))?;
            }
            PropFindSelector::Props(props) => {
                writer.write_event(Event::Start(BytesStart::new("D:prop")))?;
                for prop in props {
                    prop.write_xml(writer)?;
                }
                writer.write_event(Event::End(BytesEnd::new("D:prop")))?;
            }
        }

        Ok(())
    }

    pub fn to_xml(&self) -> Result<String, quick_xml::Error> {
        let mut writer = Writer::new(Vec::new());

        let mut root = BytesStart::new("D:propfind");
        root.push_attribute(("xmlns:D", "DAV:"));

        writer.write_event(Event::Start(root))?;

        self.write_xml(&mut writer)?;

        writer.write_event(Event::End(BytesEnd::new("D:propfind")))?;

        let bytes = writer.into_inner();

        Ok(String::from_utf8(bytes).expect("XML should always be valid UTF-8"))
    }
}
