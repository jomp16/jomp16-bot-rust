use simple_irc::Prefix;

use crate::geoip_response;
use maxminddb::Reader;

pub struct PrivMsgRequest<'a> {
    pub user: &'a Prefix,
    pub source: &'a String,
    pub message: &'a String,
}

pub struct PrivMsgResponse {
    pub target: String,
    pub message: String,
}

pub trait PrivMsgEvent: Send + Sync {
    fn execute(&self, request: PrivMsgRequest) -> Option<PrivMsgResponse>;
}

pub struct GeoIpPrivMsgEvent {
    pub reader_asn: Reader<Vec<u8>>,
    pub reader_city: Reader<Vec<u8>>,
}

impl Default for GeoIpPrivMsgEvent {
    fn default() -> Self {
        GeoIpPrivMsgEvent {
            reader_asn: maxminddb::Reader::open_readfile("GeoLite2-ASN.mmdb").unwrap(),
            reader_city: maxminddb::Reader::open_readfile("GeoLite2-City.mmdb").unwrap()
        }
    }
}

impl PrivMsgEvent for GeoIpPrivMsgEvent {
    fn execute(&self, request: PrivMsgRequest) -> Option<PrivMsgResponse> {
        if request.message.starts_with(".geoip") {
            let ip_request = &request.message[7..];
            let mut message: String = "".to_string();

            match geoip_response::ip_to_geoip(vec![ip_request], &self.reader_asn, &self.reader_city) {
                Ok(vector_geoip) => {
                    if let Some(geoip) = vector_geoip.first() {
                        log::info!("IP: {}", ip_request);

                        // AS-NAME / ASN / PTR / país - estado - cidade

                        message = format!("^ {:} / {:} / {:} / {:} / {:} - {:} - {:}",
                                          geoip.asn.name,
                                          geoip.asn.number,
                                          geoip.ip.ip,
                                          geoip.ip.ptr,
                                          geoip.city.country,
                                          geoip.city.state,
                                          geoip.city.name
                        );
                    }
                }
                Err(e) => {
                    message = format!("An error happened while geolocating IP: {}, message: {}", ip_request, e);
                }
            }

            return Some(PrivMsgResponse {
                target: request.source.clone(),
                message,
            });
        }

        None
    }
}