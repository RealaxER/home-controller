
#[derive(Debug, PartialEq,Clone)]
pub enum OtaErr {
    DownloadErr,
    NotEnoughMemoryErr,
    ServerNoReturnErr,
    LinkErr,
    NoSignalErr,
    UserCalendarErr,
    VersionErr,
    NoLinkResErr, 
    VerifyErr,
    VerifyNotEqualErr,
    CheckVersionErr,
    HttpErr,
    MqttErr,
    TimoutErr,
}