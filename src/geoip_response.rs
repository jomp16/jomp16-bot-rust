use std::time::Instant;

#[derive(Debug)]
pub struct GeoIpResponse {
    pub ip: GeoIpDataResponse,
    pub city: GeoIpCityResponse,
    pub asn: GeoIpAsnResponse,
}

#[derive(Debug)]
pub struct GeoIpDataResponse {
    pub ip: String,
    pub ptr: String,
}

#[derive(Debug)]
pub struct GeoIpCityResponse {
    pub name: String,
    pub state: String,
    pub country: String,
    pub country_iso_code: String,
}

#[derive(Debug)]
pub struct GeoIpAsnResponse {
    pub number: String,
    pub name: String,
}

pub fn ip_to_geoip(ips: Vec<&str>, reader_asn: &maxminddb::Reader<Vec<u8>>, reader_city: &maxminddb::Reader<Vec<u8>>) -> Result<Vec<GeoIpResponse>, std::io::Error> {
    let mut array_geoip: Vec<GeoIpResponse> = vec![];

    for ip_addr in ips.iter() {
        let now = Instant::now();

        log::info!("Geolocating IP {}", ip_addr);

        let ip_result = dns_lookup::lookup_host(ip_addr);

        if let Err(e) = ip_result {
            log::info!("Cannot resolve IP for domain: {}, error: {}", ip_addr, e);

            return Err(e);
        }

        let ip = *ip_result.unwrap().first().unwrap();

        if ip.to_string().ne(ip_addr) {
            log::info!("Resolved DNS {} to IP {}", ip_addr, ip.to_string())
        }
        let ptr_dns = dns_lookup::lookup_addr(&ip).unwrap().to_string();
        let ptr = if ptr_dns.eq(&ip.to_string()) { "No PTR".to_string() } else { ptr_dns };
        let asn_option: Result<maxminddb::geoip2::Asn, maxminddb::MaxMindDBError> = reader_asn.lookup(ip);
        let city_option: Result<maxminddb::geoip2::City, maxminddb::MaxMindDBError> = reader_city.lookup(ip);

        let mut city_name: String = "No City".to_string();
        let mut state_name: String = "No State".to_string();
        let mut country_name: String = "No Country".to_string();
        let mut country_iso_code: String = "No Country".to_string();

        let mut asn_number: String = "No ASN".to_string();
        let mut asn_name: String = "No ASN".to_string();

        match city_option {
            Ok(city) => {
                if let Some(i) = city.city {
                    city_name = i.names.as_ref().unwrap().get("en").unwrap().to_string()
                } else {
                    log::error!("No City found for IP: {}", ip);
                }

                if let Some(i) = &city.subdivisions {
                    state_name = i.first().unwrap().names.as_ref().unwrap().get("en").unwrap().to_string()
                } else {
                    log::error!("No State found for IP: {}", ip);
                }

                if let Some(i) = &city.country {
                    country_name = i.names.as_ref().unwrap().get("en").unwrap().to_string()
                } else {
                    log::error!("No Country found for IP: {}", ip);
                }

                if let Some(i) = &city.country {
                    country_iso_code = i.iso_code.unwrap().to_owned()
                } else {
                    log::error!("No Country ISO code found for IP: {}", ip);
                }
            }
            Err(err) => log::error!("An error happened while searching City for IP: {}, {}", ip, err),
        }

        match asn_option {
            Ok(asn) => {
                asn_number = format!("AS{}", asn.autonomous_system_number.unwrap().to_string());
                asn_name = asn.autonomous_system_organization.unwrap().to_string();
            }
            Err(err) => log::error!("An error happened while searching ASN for IP: {}, {}", ip, err),
        }

        let response = GeoIpResponse {
            ip: GeoIpDataResponse {
                ip: (ip.to_string()).parse().unwrap(),
                ptr: ptr.to_string(),
            },
            city: GeoIpCityResponse {
                name: city_name,
                state: state_name,
                country: country_name,
                country_iso_code,
            },
            asn: GeoIpAsnResponse {
                number: asn_number,
                name: asn_name,
            },
        };

        array_geoip.push(response);

        log::info!("Done geolocalization of IP: {}. Elapsed time: {:?}", ip, now.elapsed());
    }

    return Ok(array_geoip);
}